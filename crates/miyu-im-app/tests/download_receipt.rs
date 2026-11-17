//! 下载的回执（施工 I-247，2026-11-17 项目主人验收第 35 条）：真核心拉起真桥、假 NapCat。终端管理员私聊里叫她传一个文件，她用
//! `qq_send_file` 传了；对方下载了，NapCat 推来一条对方发的、只带这个文件的私聊消息：记进聊天记录、不开一轮。对方真发来一个
//! 别的文件、再说一句的，照常开一轮。

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};
use tokio::sync::oneshot;

use crate::support::group::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 文件的内容：7 个字节。
const VOICE: &[u8] = b"ID3fake";

/// 一个文件段：名字 `name`、`size` 个字节。
fn file(name: &str, size: usize) -> Value {
    json!({"type": "file", "data": {"file": name, "file_id": format!("{name}-id"), "file_size": size.to_string()}})
}

/// 终端管理员本人的私聊（不关在沙盒里）：等到有了，交回它的事件。
async fn mine(home: &Home, wanted: impl Fn(&[Value]) -> bool) -> Vec<Value> {
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        if let Some(events) = home
            .own_sessions()
            .first()
            .map(|session| home.events(session))
            .filter(|events| wanted(events))
        {
            return events;
        }
        assert!(tokio::time::Instant::now() < deadline, "等不到私聊会话");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

/// 下一个发给终端管理员的动作：传文件的交回 `upload`，说话的交回那句字。
async fn next_private(napcat: &mut Answering) -> String {
    loop {
        let action = napcat.action().await;
        match action["action"].as_str() {
            Some("upload_private_file") => return "upload".to_string(),
            Some("send_private_msg") => {
                return action["params"]["message"][0]["data"]["text"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
            }
            _ => {}
        }
    }
}

#[tokio::test]
async fn a_download_receipt_of_her_file_does_not_open_a_turn() {
    let (release, hold) = oneshot::channel();
    let lines = Lines::new([
        Line::uses("", "qq_send_file", r#"{"path":"say-01.mp3"}"#).released_by(hold),
        Line::says("发了"),
        Line::says("收到你的文件了"),
        Line::says("在"),
    ]);
    let (home, mut napcat, _) = started_showing(Arc::new(lines.clone()), "", |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    napcat.send(private_frame(ADMIN, 1, json!([plain("发个语音")])));
    let created = mine(&home, |events| !events.is_empty()).await;
    let cwd = created[0]["body"]["cwd"]
        .as_str()
        .map(PathBuf::from)
        .expect("有工作目录");
    std::fs::create_dir_all(&cwd).expect("建得了目录");
    std::fs::write(cwd.join("say-01.mp3"), VOICE).expect("写得进");
    release.send(()).expect("还在等");
    assert_eq!(next_private(&mut napcat).await, "upload");
    assert_eq!(next_private(&mut napcat).await, "发了");
    until_log(&home, "file shown").await;
    // 对方下载了：只带那个文件（名字、字节数一样）。
    napcat.send(private_frame(
        ADMIN,
        2,
        json!([file("say-01.mp3", VOICE.len())]),
    ));
    until_log(&home, "download receipt, not taken").await;
    // 对方真发来一个别的文件：照常开一轮。
    napcat.send(private_frame(ADMIN, 3, json!([file("notes.md", 12)])));
    assert_eq!(next_private(&mut napcat).await, "收到你的文件了");
    napcat.send(private_frame(ADMIN, 4, json!([plain("在吗")])));
    assert_eq!(next_private(&mut napcat).await, "在");
    let events = mine(&home, |events| {
        events
            .iter()
            .filter(|event| event["kind"] == "turn.ended")
            .count()
            == 3
    })
    .await;
    let heard: Vec<_> = said(&events)
        .iter()
        .map(|event| event["body"]["venue"]["msg"].clone())
        .collect();
    assert_eq!(heard, [json!("1"), json!("3"), json!("4")], "回执不进会话");
    stopped(home).await;
}

/// 等到运行日志里有 `wanted`。
async fn until_log(home: &Home, wanted: &str) {
    let deadline = tokio::time::Instant::now() + WAIT;
    while !run_log(&home.root).contains(wanted) {
        assert!(
            tokio::time::Instant::now() < deadline,
            "等不到 {wanted}：{}",
            run_log(&home.root)
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}
