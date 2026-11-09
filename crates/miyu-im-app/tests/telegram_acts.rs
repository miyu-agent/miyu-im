//! Telegram 的撤回、贴表情（施工 I-185 下上，设计 07 第 3.5 节「说话」那一块）：真核心拉起真程序，Bot API 指本机的假服务器，模型照
//! 剧本调。`tg_react` 照 `setMessageReaction` 在那一条上贴一个表情，`tg_recall` 照 `deleteMessage` 撤；她给的编号 im 的记录里没有的
//! 答 not-found、不去调；没给编号、叫她的那条也没引用的答 no-quote。判过要回的那一条上宿主自己贴 Telegram 的 👀（驱动给的，
//! 不是 `bridge.json` 里 QQ 的表情编号）。

use serde_json::json;

use miyu_session::testkit::{Play, Script};

use crate::support::bot_api::{BotApi, Mode};
use crate::support::events::{of_kind, until_events};
use crate::support::telegram::{ALICE, GROUP, WHEN_CALLED, said, started_ruled, until_tg, venue};
use crate::support::*;

/// Telegram 的那一句 `telegram/tool-results/<name>.txt`，去掉行尾。
fn tg_sentence(name: &str) -> String {
    let path = resources().join(format!(
        "packages/miyu-im/data/telegram/tool-results/{name}.txt"
    ));
    std::fs::read_to_string(path)
        .expect("读得出")
        .trim_end()
        .to_string()
}

#[tokio::test]
async fn she_reacts_to_and_recalls_messages_in_the_group() {
    let api = BotApi::start(Mode::Normal).await;
    let script = Script::new([
        Play::calls(&[("tg_react", r#"{"msg":"50","emoji":"👍"}"#)]),
        Play::calls(&[("tg_recall", r#"{"msg":"50"}"#)]),
        Play::calls(&[("tg_recall", "{}")]),
        Play::calls(&[("tg_react", r#"{"msg":"999","emoji":"👍"}"#)]),
        Play::Says("好了"),
    ]);
    let cards = r#"{"others": {"tg": {"groups": {"on": true, "final_only": false}}}}"#;
    let (home, mut open) = started_ruled(&api, script, (Some(cards), ""), WHEN_CALLED).await;
    until_tg(&mut open, |one| one["connected"] == true).await;
    api.push(said(1, 50, (ALICE, "Alice"), "今晚吃火锅", false));
    until_events(&home.root, &venue(), |events| {
        !of_kind(events, "message.user").is_empty()
    })
    .await;
    api.push(said(
        2,
        51,
        (ADMIN, "Shorin"),
        "@miyu_test_bot 处理一下",
        true,
    ));
    api.until(|method, params| method == "sendMessage" && params["text"] == "好了")
        .await;
    let reacted = api
        .until(|method, params| method == "setMessageReaction" && params["message_id"] == 50)
        .await;
    assert_eq!(
        reacted,
        json!({"chat_id": GROUP.to_string(), "message_id": 50, "reaction": [{"type": "emoji", "emoji": "👍"}]})
    );
    // 判过要回的那一条上宿主自己贴的是 Telegram 的 👀，不是 QQ 的表情编号（`bridge.json` 的 289，开了贴表情以后贴过）。
    let seen = api
        .until(|method, params| method == "setMessageReaction" && params["message_id"] == 51)
        .await;
    assert_eq!(
        seen["reaction"],
        json!([{"type": "emoji", "emoji": "👀"}]),
        "{seen}"
    );
    let deleted = api.until(|method, _| method == "deleteMessage").await;
    assert_eq!(
        deleted,
        json!({"chat_id": GROUP.to_string(), "message_id": 50})
    );
    let calls = api.calls();
    let on_50 = calls
        .iter()
        .filter(|(method, params)| {
            (method == "setMessageReaction" || method == "deleteMessage")
                && params["message_id"] != 51
        })
        .count();
    assert_eq!(on_50, 2, "记录里没有的、没给编号的都没去调：{calls:?}");
    let events = until_events(&home.root, &venue(), |events| {
        of_kind(events, "tool.result").len() >= 4
    })
    .await;
    let results: Vec<String> = of_kind(&events, "tool.result")
        .iter()
        .map(|result| {
            result["body"]["blocks"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .trim_end()
                .to_string()
        })
        .collect();
    assert_eq!(
        results,
        [
            tg_sentence("reacted"),
            tg_sentence("recalled"),
            tg_sentence("no-quote"),
            tg_sentence("not-found")
        ]
    );
    home.stop_extensions().await;
}
