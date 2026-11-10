//! 传文件（施工 I-130）：真核心照开关拉起真桥、假 NapCat 从本机连进来、她照台词说。群里她调 `qq_send_file`：工作区里的文件钉进
//! im 状态目录的 `qq/outbox/`，把那一份的路径交给 NapCat 的 `upload_group_file`（不读进来、不转 base64），当场答在传了，传完删掉。
//! 工作区外面的：群友叫的不传，答不是工作区里的文件；主人本人叫的、完全放开的会话照传（施工 LV-4，项目主人 11-10 定），交的也是
//! 钉进 `outbox/` 的那一份，传完删的不是原文件。哪个系统都传（施工 I-185 下下，项目主人 11-10：别管什么身份）。

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};
use tokio::sync::oneshot;

use crate::support::group::*;
use crate::support::platform::{GROUP, MEMBER, venue};
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 假 NapCat 认得的群成员：她自己、终端管理员、一个群友。
const MEMBERS: &[Member] = &[
    (BOT, "米尤", "miyu"),
    (ADMIN, "老板", "boss"),
    (MEMBER, "小明", "member"),
];

/// 系统的场所规则：叫她才回（不过判官）。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\ndiscipline = \"when-called\"\n";

/// 她第一步传两份（工作区里的、工作区外面的），等 `hold` 放行；第二步说「好了」。
fn lines(hold: oneshot::Receiver<()>) -> Lines {
    Lines::new([
        Line::uses("传给你", "qq_send_file", r#"{"path":"out/report.pdf"}"#)
            .and("qq_send_file", r#"{"path":"../outside.txt"}"#)
            .released_by(hold),
        Line::says("好了"),
    ])
}

/// 等到群会话有了：交回它的工作目录（`session.created` 的 `cwd`）。
async fn until_group_cwd(home: &Home) -> PathBuf {
    let events = until_events(&home.root, &venue(), |events| !events.is_empty()).await;
    events[0]["body"]["cwd"]
        .as_str()
        .map(PathBuf::from)
        .expect("群会话有工作目录")
}

/// 一次工具结果的字。
fn result_text(event: &Value) -> String {
    event["body"]["blocks"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// `who`（号、名字、身份）在群里 @ 她叫她传：交回数据根、假 NapCat、她的工作目录（工作区里、外面各放好一份，放好了才让她调）。
async fn asked_by(who: (i64, &str, &str)) -> (Home, Answering, PathBuf) {
    let (release, hold) = oneshot::channel();
    let lines = lines(hold);
    let (home, napcat, _) = started_showing(Arc::new(lines.clone()), RULES, |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    let (id, name, role) = who;
    napcat.send(group_frame(
        GROUP,
        id,
        1,
        json!([at(BOT), plain(" 传一下")]),
        (name, role),
    ));
    let cwd = until_group_cwd(&home).await;
    std::fs::create_dir_all(cwd.join("out")).expect("建得了目录");
    std::fs::write(cwd.join("out/report.pdf"), b"%PDF-1.7").expect("写得进");
    std::fs::write(cwd.parent().expect("有上一级").join("outside.txt"), b"key").expect("写得进");
    release.send(()).expect("还在等");
    (home, napcat, cwd)
}

/// 下一个传文件的动作。
async fn next_upload(napcat: &mut Answering) -> Value {
    loop {
        let action = napcat.action().await;
        if action["action"] == "upload_group_file" {
            return action;
        }
    }
}

/// 等到两次调用都答了：交回答的字。
async fn two_results(home: &Home) -> Vec<String> {
    let events = until_events(&home.root, &venue(), |events| {
        events
            .iter()
            .filter(|event| event["kind"] == "tool.result")
            .count()
            >= 2
    })
    .await;
    events
        .iter()
        .filter(|event| event["kind"] == "tool.result")
        .map(result_text)
        .collect()
}

/// im 钉文件的目录（真实位置）。
fn outbox(home: &Home) -> PathBuf {
    std::fs::canonicalize(miyu_im::dirs::state(&home.root))
        .expect("有状态目录")
        .join("qq/outbox")
}

#[tokio::test]
async fn a_member_gets_only_the_file_in_her_workspace_sent() {
    let (home, mut napcat, _) = asked_by((MEMBER, "小明", "member")).await;
    let upload = next_upload(&mut napcat).await;
    assert_eq!(upload["params"]["group_id"], GROUP);
    assert_eq!(upload["params"]["name"], "report.pdf");
    let file = PathBuf::from(upload["params"]["file"].as_str().expect("是路径"));
    assert!(
        file.starts_with(outbox(&home)),
        "交的是钉住的那一份，不转 base64：{}",
        file.display()
    );
    let results = two_results(&home).await;
    assert!(
        results
            .iter()
            .any(|text| text.starts_with("Uploading report.pdf (8 B)")),
        "{results:?}"
    );
    assert!(
        results
            .iter()
            .any(|text| text.contains("../outside.txt is not a file in your workspace")),
        "群友叫的：外面的不传。{results:?}"
    );
    // 传完删掉钉住的那一份：发出去了才写「file shown」。
    until_logged(&home.root, |log| log.contains("file shown")).await;
    assert!(!file.exists(), "传完删掉");
    stopped(home).await;
}

#[tokio::test]
async fn the_owner_in_a_fully_open_session_gets_files_outside_sent_too() {
    // 施工 LV-4：主人本人叫的、核心交的权限是完全放开，工作区外面的也传；交的是钉进 `outbox/` 的那一份，原文件留着。
    let (home, mut napcat, cwd) = asked_by((ADMIN, "老板", "boss")).await;
    let mut names = Vec::new();
    for _ in 0..2 {
        let upload = next_upload(&mut napcat).await;
        let file = PathBuf::from(upload["params"]["file"].as_str().expect("是路径"));
        assert!(file.starts_with(outbox(&home)), "{}", file.display());
        names.push(
            upload["params"]["name"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        );
    }
    names.sort();
    assert_eq!(names, ["outside.txt", "report.pdf"]);
    let results = two_results(&home).await;
    assert!(
        results
            .iter()
            .any(|text| text.starts_with("Uploading outside.txt (3 B)")),
        "{results:?}"
    );
    until_logged(&home.root, |log| log.matches("file shown").count() >= 2).await;
    let outside = cwd.parent().expect("有上一级").join("outside.txt");
    assert_eq!(std::fs::read(outside).expect("原文件还在"), b"key");
    stopped(home).await;
}
