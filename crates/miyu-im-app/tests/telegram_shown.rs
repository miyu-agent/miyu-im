//! 她给人看的图、文件发进 Telegram（施工 I-185 中，设计 07 第 3.5 节「发图」）：真核心拉起真程序，Bot API 指本机的假服务器，工具
//! 目录里有基础系统的 `show`。主人私聊里她调 `show`：图照 `sendPhoto` 传（`multipart/form-data`，内容是文件原样），别的照
//! `sendDocument` 传、带文件名；都回在那个私聊里。

use std::path::PathBuf;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::json;

use miyu_session::testkit::{Play, Script};

use crate::support::bot_api::{BotApi, Mode};
use crate::support::media::png;
use crate::support::telegram::{started_showing, until_tg};
use crate::support::*;

/// 等到有了会话：交回它的工作目录（`session.created` 的 `cwd`）。她调 `show` 照它找相对路径。
async fn until_cwd(home: &Home) -> PathBuf {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
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

#[tokio::test]
async fn a_picture_and_a_file_she_shows_reach_the_telegram_chat() {
    let api = BotApi::start(Mode::Normal).await;
    let picture = png(3, 2);
    let script = Script::new([
        Play::Says("先等等"),
        Play::calls(&[
            ("show", r#"{"path":"pic.png"}"#),
            ("show", r#"{"path":"note.txt"}"#),
        ]),
        Play::Says("好了"),
    ]);
    let cards = r#"{"others": {"tg": {"privates": {"on": true}}}}"#;
    let (home, mut open) = started_showing(&api, script.clone(), cards).await;
    until_tg(&mut open, |one| one["connected"] == true).await;
    let said = |update: i64, id: i64, text: &str| {
        json!({"update_id": update, "message": {
            "message_id": id, "date": 1_700_000_000 + id,
            "chat": {"id": ADMIN, "type": "private"},
            "from": {"id": ADMIN, "is_bot": false, "first_name": "Shorin"},
            "text": text,
        }})
    };
    // 头一条开出会话，放好要给人看的两份，第二条叫她发。
    api.push(said(1, 7, "在吗"));
    let cwd = until_cwd(&home).await;
    std::fs::create_dir_all(&cwd).expect("建得了目录");
    std::fs::write(cwd.join("pic.png"), &picture).expect("写得进");
    std::fs::write(cwd.join("note.txt"), "周六上午十点，东门集合。").expect("写得进");
    api.until(|method, params| method == "sendMessage" && params["text"] == "先等等")
        .await;
    api.push(said(2, 8, "发一下"));
    let photo = api.until(|method, _| method == "sendPhoto").await;
    assert_eq!(photo["chat_id"], ADMIN.to_string(), "{photo}");
    assert_eq!(
        photo["photo"]["base64"],
        STANDARD.encode(&picture),
        "原样传"
    );
    let document = api.until(|method, _| method == "sendDocument").await;
    assert_eq!(document["chat_id"], ADMIN.to_string(), "{document}");
    assert_eq!(document["document"]["filename"], "note.txt");
    assert_eq!(
        document["document"]["base64"],
        STANDARD.encode("周六上午十点，东门集合。")
    );
    let log = crate::support::events::until_logged(&home.root, |log| {
        log.matches("file shown").count() >= 2
    })
    .await;
    assert_eq!(log.matches("file shown").count(), 2, "{log}");
    home.stop_extensions().await;
}
