//! Telegram 收图（施工 I-185 上，设计 07 第 3.5 节「收图给她看」）：真核心拉起真程序，Bot API 指本机的假服务器。叫她的那一条带的图、
//! 它引用的那一条带的图都经 `getFile` 取回来、交给她看（引用的在前）；引用的那一条照收的时候记下的答（Telegram 不给机器人取消息），
//! 运行日志里没有 `picture not fetched`。

use serde_json::json;

use miyu_session::testkit::{Play, Script};

use crate::support::bot_api::{BotApi, Mode};
use crate::support::events::run_log;
use crate::support::media::{has_blob, pictures, png};
use crate::support::telegram::{ALICE, GROUP, WHEN_CALLED, started_ruled, until_tg};
use crate::support::*;

#[tokio::test]
async fn the_pictures_of_the_message_calling_her_and_of_the_quoted_one_reach_her() {
    let api = BotApi::start(Mode::Normal).await;
    let (quoted, own) = (png(2, 2), png(4, 3));
    api.file("pic-q", quoted.clone());
    api.file("pic-a", own.clone());
    let script = Script::new([Play::Says("看到了")]);
    let cards = r#"{"others": {"tg": {"groups": {"on": true, "final_only": false}}}}"#;
    let (home, mut open) =
        started_ruled(&api, script.clone(), (Some(cards), ""), WHEN_CALLED).await;
    until_tg(&mut open, |one| one["connected"] == true).await;
    let chat = json!({"id": GROUP, "type": "supergroup", "title": "Miyu 测试群"});
    let earlier = json!({
        "message_id": 60, "date": 1_700_000_060, "chat": chat,
        "from": {"id": ALICE, "is_bot": false, "first_name": "Alice"},
        "photo": [{"file_id": "pic-q-small"}, {"file_id": "pic-q", "file_size": quoted.len()}],
        "caption": "这张",
    });
    // 旁听的那一条：只记下，不取图。
    api.push(json!({"update_id": 1, "message": earlier}));
    // 主人引用它、@ 她，自己也带一张。
    api.push(json!({"update_id": 2, "message": {
        "message_id": 61, "date": 1_700_000_061, "chat": chat,
        "from": {"id": ADMIN, "is_bot": false, "first_name": "Shorin"},
        "photo": [{"file_id": "pic-a", "file_size": own.len()}],
        "caption": "@miyu_test_bot 看看",
        "caption_entities": [{"type": "mention", "offset": 0, "length": 14}],
        "reply_to_message": earlier,
    }}));
    api.until(|method, params| method == "sendMessage" && params["text"] == "看到了")
        .await;
    let (_, request) = script.requests().into_iter().next().expect("请求过模型");
    let seen = pictures(&request);
    let names: Vec<&str> = seen.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["msg-60-1", "msg-61-1"], "{request:?}");
    assert!(
        has_blob(&home, &quoted) && has_blob(&home, &own),
        "两张都存成了 blob"
    );
    let downloads = api
        .calls()
        .iter()
        .filter(|(method, _)| method == "download")
        .count();
    assert_eq!(
        downloads,
        2,
        "旁听时没取，叫她时各取一次：{:?}",
        api.calls()
    );
    let log = run_log(&home.root);
    assert!(!log.contains("picture not fetched"), "{log}");
    home.stop_extensions().await;
}
