//! 传文件（施工 I-130）：真核心照开关拉起真桥、假 NapCat 从本机连进来、她照台词说。群里她调 `qq_send_file`：工作区里的文件把
//! 真实路径原样交给 NapCat 的 `upload_group_file`（不读进来、不转 base64），当场答在传了；工作区外面的不传，答不是工作区里的文件。

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};
use tokio::sync::oneshot;

use crate::support::group::*;
use crate::support::platform::{GROUP, venue};
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 假 NapCat 认得的群成员：她自己、终端管理员。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (ADMIN, "老板", "boss")];

/// 系统的场所规则：终端管理员 @ 她照回。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\n";

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

#[tokio::test]
async fn a_file_in_her_workspace_goes_to_napcat_by_path() {
    let (release, hold) = oneshot::channel();
    let lines = lines(hold);
    let (home, mut napcat, _) = started_showing(Arc::new(lines.clone()), RULES, |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    napcat.send(group_frame(
        GROUP,
        ADMIN,
        1,
        json!([at(BOT), plain(" 传一下")]),
        ("老板", "boss"),
    ));
    let cwd = until_group_cwd(&home).await;
    std::fs::create_dir_all(cwd.join("out")).expect("建得了目录");
    std::fs::write(cwd.join("out/report.pdf"), b"%PDF-1.7").expect("写得进");
    std::fs::write(cwd.parent().expect("有上一级").join("outside.txt"), b"key").expect("写得进");
    release.send(()).expect("还在等");
    let upload = loop {
        let action = napcat.action().await;
        if action["action"] == "upload_group_file" {
            break action;
        }
    };
    let real = std::fs::canonicalize(cwd.join("out/report.pdf")).expect("在");
    assert_eq!(
        upload["params"],
        json!({"group_id": GROUP, "file": real.to_string_lossy(), "name": "report.pdf"}),
        "真实路径原样交，不转 base64"
    );
    let events = until_events(&home.root, &venue(), |events| {
        events
            .iter()
            .filter(|event| event["kind"] == "tool.result")
            .count()
            >= 2
    })
    .await;
    let results: Vec<String> = events
        .iter()
        .filter(|event| event["kind"] == "tool.result")
        .map(result_text)
        .collect();
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
        "{results:?}"
    );
    stopped(home).await;
}
