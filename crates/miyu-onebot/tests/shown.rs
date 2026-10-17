//! 她给人看的文件发进 QQ（施工 O-46，`onebot.md` 第一条「展示」）：真核心照开关拉起真桥、假 NapCat、她照台词说
//! （`support/speaking.rs`）。终端管理员私聊里她调核心的 `show`：图片发成图片段（`base64://` 是文件的原样），别的传成文件（带
//! 文件名，比 `blob.get` 的一块大的分几块读）；NapCat 回失败的不另发一句，给那个会话记一块 `unshown` 事实（`context.injected`），写着为什么和文件名。
//!
//! 群里（施工 O-46 补）：终端管理员在群里 @ 她，她在群会话里调 `show`，图片发成群里的图片段、别的传成群文件。群会话归系统账号，
//! blob 照 `blob.get {blob, session}` 读（核心 W-6 补）；W-6 补以前的核心回 `unknown_blob`，这一条是红的。她被禁言着的不发，
//! 记 `unshown`（`why="muted"`）。

use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use tokio::sync::oneshot;

use crate::support::group::*;
use crate::support::platform::{GROUP, venue};
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 一张 1×1 的 PNG。
const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0,
    0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

/// 一份文本的一行：重复成一份比 `blob.get` 的一块（512 KiB）大的文本，要分几块读。
const LINE: &str = "周六上午十点，东门集合。\n";

/// 那份文本：约 600 KiB。
fn note() -> String {
    LINE.repeat(600 * 1024 / LINE.len())
}

/// 假 NapCat 认得的群成员：她自己、终端管理员。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (ADMIN, "老板", "boss")];

/// 系统的场所规则：群都不抽样，终端管理员 @ 她照回。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0 }\n";

/// 她第一步调两次 `show`（一张图、一份文本），等 `hold` 放行；第二步说「好了」。
fn lines(hold: oneshot::Receiver<()>) -> Lines {
    Lines::new([
        Line::uses("给你看", "show", r#"{"path":"pic.png"}"#)
            .and("show", r#"{"path":"note.txt","title":"集合"}"#)
            .released_by(hold),
        Line::says("好了"),
    ])
}

/// 等到管理员名下有了会话：交回它的工作目录（`session.created` 的 `cwd`）。她调 `show` 照它找相对路径。
async fn until_cwd(home: &Home) -> PathBuf {
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        let cwd = home.sessions().iter().find_map(|session| {
            home.events(session)
                .first()
                .and_then(|created| created["body"]["cwd"].as_str().map(PathBuf::from))
        });
        if let Some(cwd) = cwd {
            return cwd;
        }
        assert!(tokio::time::Instant::now() < deadline, "等不到会话");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

/// 等到群会话有了：交回它的工作目录（`session.created` 的 `cwd`）。
async fn until_group_cwd(home: &Home) -> PathBuf {
    let events = until_events(&home.root, &venue(), |events| !events.is_empty()).await;
    events[0]["body"]["cwd"]
        .as_str()
        .map(PathBuf::from)
        .expect("群会话有工作目录")
}

/// 在工作目录 `cwd` 里放好那张图、那份文本。
fn stock(cwd: &PathBuf) {
    std::fs::create_dir_all(cwd).expect("建得了目录");
    std::fs::write(cwd.join("pic.png"), PNG).expect("写得进");
    std::fs::write(cwd.join("note.txt"), note()).expect("写得进");
}

/// 管理员名下全部会话的事件。
fn all_events(home: &Home) -> Vec<Value> {
    home.sessions()
        .iter()
        .flat_map(|session| home.events(session))
        .collect()
}

#[tokio::test]
async fn a_picture_and_a_file_she_shows_reach_the_private_chat() {
    let (release, hold) = oneshot::channel();
    let lines = lines(hold);
    let (home, mut napcat, _) = started_showing(Arc::new(lines.clone()), RULES, |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    napcat.send(private_frame(ADMIN, 1, json!([plain("看看")])));
    stock(&until_cwd(&home).await);
    release.send(()).expect("还在等");
    // 两句话、一张图、一份文件：图和文件各起一个任务发，先后不一定。
    let mut actions = Vec::new();
    for _ in 0..4 {
        actions.push(napcat.action().await);
    }
    let image = actions
        .iter()
        .find(|action| action["params"]["message"][0]["type"] == "image")
        .unwrap_or_else(|| panic!("没有图：{actions:#?}"));
    assert_eq!(image["action"], "send_private_msg");
    assert_eq!(image["params"]["user_id"], ADMIN);
    assert_eq!(
        image["params"]["message"][0]["data"]["file"],
        format!("base64://{}", STANDARD.encode(PNG))
    );
    let upload = actions
        .iter()
        .find(|action| action["action"] == "upload_private_file")
        .unwrap_or_else(|| panic!("没有文件：{actions:#?}"));
    assert_eq!(
        upload["params"],
        json!({"user_id": ADMIN, "file": format!("base64://{}", STANDARD.encode(note())), "name": "note.txt"})
    );
    let texts: Vec<&str> = actions
        .iter()
        .filter(|action| action["params"]["message"][0]["type"] == "text")
        .filter_map(|action| action["params"]["message"][0]["data"]["text"].as_str())
        .collect();
    assert_eq!(texts, ["给你看", "好了"]);
    let log = run_log(&home.root);
    assert_eq!(log.matches("file shown").count(), 2, "{log}");
    stopped(home).await;
}

#[tokio::test]
async fn a_file_napcat_refuses_is_noted_where_she_will_see_it() {
    let (release, hold) = oneshot::channel();
    let lines = lines(hold);
    let (home, napcat, _) = started_showing(Arc::new(lines.clone()), RULES, |napcat| {
        napcat.refusing(MEMBERS)
    })
    .await;
    napcat.send(private_frame(ADMIN, 1, json!([plain("看看")])));
    stock(&until_cwd(&home).await);
    release.send(()).expect("还在等");
    let deadline = tokio::time::Instant::now() + WAIT;
    let unshown = loop {
        let found: Vec<Value> = all_events(&home)
            .into_iter()
            .filter(|event| {
                event["kind"] == "context.injected" && event["body"]["kind"] == "unshown"
            })
            .collect();
        if found.len() == 2 {
            break found;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "等不到两块：{found:#?}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    let texts: Vec<&str> = unshown
        .iter()
        .filter_map(|event| event["body"]["text"].as_str())
        .collect();
    for name in ["pic.png", "note.txt"] {
        assert!(
            texts.iter().any(
                |text| text.contains("why=\"failed\"") && text.contains(&format!("\"{name}\""))
            ),
            "{texts:#?}"
        );
    }
    stopped(home).await;
}

#[tokio::test]
async fn a_picture_and_a_file_she_shows_reach_the_group() {
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
        json!([at(BOT), plain(" 看看")]),
        ("老板", "boss"),
    ));
    stock(&until_group_cwd(&home).await);
    release.send(()).expect("还在等");
    // 两句话、一张图、一份文件，先后不一定。
    let mut actions: Vec<Value> = Vec::new();
    let wanted = |actions: &[Value]| {
        let image = actions
            .iter()
            .any(|action| action["params"]["message"][0]["type"] == "image");
        let upload = actions
            .iter()
            .any(|action| action["action"] == "upload_group_file");
        // 第二句前面带引用（群里最后一条是她自己的），文字段不一定在头一个。
        let texts = actions
            .iter()
            .filter(|action| {
                action["params"]["message"]
                    .as_array()
                    .is_some_and(|message| message.iter().any(|piece| piece["type"] == "text"))
            })
            .count();
        image && upload && texts == 2
    };
    while !wanted(&actions) {
        assert!(actions.len() < 10, "{actions:#?}");
        actions.push(napcat.action().await);
    }
    let image = actions
        .iter()
        .find(|action| action["params"]["message"][0]["type"] == "image")
        .expect("有图");
    assert_eq!(image["action"], "send_group_msg");
    assert_eq!(image["params"]["group_id"], GROUP);
    assert_eq!(
        image["params"]["message"][0]["data"]["file"],
        format!("base64://{}", STANDARD.encode(PNG))
    );
    let upload = actions
        .iter()
        .find(|action| action["action"] == "upload_group_file")
        .expect("有文件");
    assert_eq!(
        upload["params"],
        json!({"group_id": GROUP, "file": format!("base64://{}", STANDARD.encode(note())), "name": "note.txt"})
    );
    let log = run_log(&home.root);
    assert_eq!(log.matches("file shown").count(), 2, "{log}");
    assert!(!log.contains("unknown_blob"), "{log}");
    stopped(home).await;
}

#[tokio::test]
async fn files_she_shows_while_muted_are_noted_not_sent() {
    let (release, hold) = oneshot::channel();
    let lines = lines(hold);
    let (home, napcat, _) = started_showing(Arc::new(lines.clone()), RULES, |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    napcat.send(group_frame(
        GROUP,
        ADMIN,
        1,
        json!([at(BOT), plain(" 看看")]),
        ("老板", "boss"),
    ));
    stock(&until_group_cwd(&home).await);
    // 她这一轮开了、还没调 `show`，被禁言了：禁言记下以后再放行。
    napcat.send(group_ban(GROUP, BOT, "ban", Some(600)));
    until_event(&home.root, &venue(), |event| {
        event["kind"] == "ext.onebot.venues.muted"
    })
    .await;
    release.send(()).expect("还在等");
    let events = until_events(&home.root, &venue(), |events| {
        events
            .iter()
            .filter(|event| {
                event["kind"] == "context.injected" && event["body"]["kind"] == "unshown"
            })
            .count()
            == 2
    })
    .await;
    for event in events
        .iter()
        .filter(|event| event["kind"] == "context.injected" && event["body"]["kind"] == "unshown")
    {
        let text = event["body"]["text"].as_str().unwrap_or_default();
        assert!(text.contains("why=\"muted\""), "{text}");
    }
    let log = run_log(&home.root);
    assert!(!log.contains("file shown"), "{log}");
    stopped(home).await;
}
