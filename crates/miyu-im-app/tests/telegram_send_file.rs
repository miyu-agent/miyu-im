//! Telegram 传文件（施工 I-185 下下，宿主的 `common::send_file`，同 QQ 的 `qq_send_file`）：真核心拉起真程序，Bot API 指本机的假
//! 服务器。私聊里她调 `tg_send_file`：工作区里的文件钉进 `outbox/`，驱动读了照 `sendDocument` 传、带文件名，传完删掉钉的那一份；
//! 工作区外面的（这个私聊不是主人本人的，没放开）不传，答不是工作区里的文件。

use std::path::PathBuf;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::json;

use miyu_session::testkit::{Play, Script};

use crate::support::bot_api::{BotApi, Mode};
use crate::support::events::{of_kind, until_logged};
use crate::support::telegram::{started_with_cards, until_tg};
use crate::support::*;

/// 私聊的人（不是主人）。
const FRIEND: i64 = 30005;

/// 等到有了会话：交回它的工作目录（`session.created` 的 `cwd`）。
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
async fn a_file_in_her_workspace_goes_to_telegram_as_a_document() {
    let api = BotApi::start(Mode::Normal).await;
    let script = Script::new([
        Play::Says("先等等"),
        Play::calls(&[
            ("tg_send_file", r#"{"path":"out/report.pdf"}"#),
            ("tg_send_file", r#"{"path":"../outside.txt"}"#),
        ]),
        Play::Says("好了"),
    ]);
    let cards = r#"{"others": {"tg": {"privates": {"on": true}}}}"#;
    let (home, mut open) = started_with_cards(&api, script, Some(cards)).await;
    until_tg(&mut open, |one| one["connected"] == true).await;
    let said = |update: i64, id: i64, text: &str| {
        json!({"update_id": update, "message": {
            "message_id": id, "date": 1_700_000_000 + id,
            "chat": {"id": FRIEND, "type": "private"},
            "from": {"id": FRIEND, "is_bot": false, "first_name": "Ann"},
            "text": text,
        }})
    };
    api.push(said(1, 7, "在吗"));
    let cwd = until_cwd(&home).await;
    std::fs::create_dir_all(cwd.join("out")).expect("建得了目录");
    std::fs::write(cwd.join("out/report.pdf"), b"%PDF-1.7").expect("写得进");
    std::fs::write(cwd.parent().expect("有上一级").join("outside.txt"), b"key").expect("写得进");
    api.until(|method, params| method == "sendMessage" && params["text"] == "先等等")
        .await;
    api.push(said(2, 8, "把报告发我"));
    let document = api.until(|method, _| method == "sendDocument").await;
    assert_eq!(document["chat_id"], FRIEND.to_string(), "{document}");
    assert_eq!(document["document"]["filename"], "report.pdf");
    assert_eq!(document["document"]["base64"], STANDARD.encode(b"%PDF-1.7"));
    api.until(|method, params| method == "sendMessage" && params["text"] == "好了")
        .await;
    let results: Vec<String> = home
        .sessions()
        .iter()
        .flat_map(|session| of_kind(&home.events(session), "tool.result"))
        .map(|result| {
            result["body"]["blocks"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    assert!(
        results
            .iter()
            .any(|text| text.starts_with("Uploading report.pdf")),
        "{results:?}"
    );
    assert!(
        results
            .iter()
            .any(|text| text.contains("../outside.txt is outside your workspace")),
        "不是主人本人的私聊：外面的不传。{results:?}"
    );
    let calls = api.calls();
    assert_eq!(
        calls
            .iter()
            .filter(|(method, _)| method == "sendDocument")
            .count(),
        1,
        "{calls:?}"
    );
    // 传完删掉钉的那一份。
    until_logged(&home.root, |log| log.contains("file shown")).await;
    let outbox = miyu_im::dirs::state(&home.root).join("tg/outbox");
    let left = std::fs::read_dir(&outbox).map(Iterator::count).unwrap_or(0);
    assert_eq!(left, 0, "钉的那一份传完删了");
    home.stop_extensions().await;
}
