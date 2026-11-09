//! Telegram 的工具（施工 I-183 下下，设计 07 第 3.5 节）：真核心拉起真程序，Bot API 指本机的假服务器，模型照剧本调 `tg_*`。
//! 群里只给 `tg_*`、不给 `qq_*`；`tg_quote` 照 im 的聊天记录认这个群里有的那一条，下一句引用它；`tg_mention` 认在这个群里说过话的人，
//! 下一句 @ 他（没用户名的挂 `text_mention`）；没说过话的、记录里没有的编号不记；`tg_chat_history` 照 im 的记录答。

use miyu_session::testkit::{Play, Script};

use crate::support::bot_api::{BotApi, Mode};
use crate::support::events::{of_kind, until_events};
use crate::support::telegram::{ALICE, WHEN_CALLED, said, started_ruled, until_tg, venue};
use crate::support::*;

/// 等这个群的第 `turn` 轮完：交回这时的全部工具结果的字，照先后。
async fn results(home: &Home, turn: usize) -> Vec<String> {
    let events = until_events(&home.root, &venue(), |events| {
        of_kind(events, "turn.ended").len() >= turn
    })
    .await;
    of_kind(&events, "tool.result")
        .iter()
        .map(|result| {
            result["body"]["blocks"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .trim_end()
                .to_string()
        })
        .collect()
}

/// Telegram 的那一句 `telegram/tool-results/<name>.txt`，字段照 `fields` 换，去掉行尾。
fn tg_sentence(name: &str, fields: &[(&str, &str)]) -> String {
    let path = resources().join(format!(
        "packages/miyu-im/data/telegram/tool-results/{name}.txt"
    ));
    let mut text = std::fs::read_to_string(path).expect("读得出");
    for (key, value) in fields {
        text = text.replace(&format!("{{{key}}}"), value);
    }
    text.trim_end().to_string()
}

#[tokio::test]
async fn in_a_group_she_quotes_mentions_and_searches_with_the_telegram_tools() {
    let api = BotApi::start(Mode::Normal).await;
    let script = Script::new([
        // 第一轮：引用 Alice 那一条、@ 她，再说。
        Play::calls(&[("tg_quote", r#"{"msg":"50"}"#)]),
        Play::calls(&[("tg_mention", r#"{"users":[20001]}"#)]),
        Play::Says("看这条"),
        // 第二轮：记录里没有的编号、没说过话的人，都不记。
        Play::calls(&[("tg_quote", r#"{"msg":"999"}"#)]),
        Play::calls(&[("tg_mention", r#"{"users":[30003]}"#)]),
        Play::Says("没人"),
        // 第三轮：查记录。
        Play::calls(&[("tg_chat_history", r#"{"query":"火锅"}"#)]),
        Play::Says("查到了"),
    ]);
    let cards = r#"{"others": {"tg": {"groups": {"on": true, "final_only": false}}}}"#;
    let (home, mut open) =
        started_ruled(&api, script.clone(), (Some(cards), ""), WHEN_CALLED).await;
    until_tg(&mut open, |one| one["connected"] == true).await;
    api.push(said(1, 50, (ALICE, "Alice"), "今晚吃火锅", false));
    until_events(&home.root, &venue(), |events| {
        !of_kind(events, "message.user").is_empty()
    })
    .await;
    api.push(said(2, 51, (ADMIN, "Shorin"), "@miyu_test_bot 看看", true));
    let first = api
        .until(|method, params| {
            method == "sendMessage"
                && params["text"]
                    .as_str()
                    .is_some_and(|text| text.ends_with("看这条"))
        })
        .await;
    assert_eq!(first["reply_parameters"]["message_id"], 50, "{first}");
    assert_eq!(first["text"], "@Alice 看这条", "{first}");
    assert_eq!(first["entities"][0]["user"]["id"], ALICE, "{first}");
    assert_eq!(
        results(&home, 1).await,
        [
            tg_sentence("quoted", &[]),
            tg_sentence("mentioned", &[("who", "Alice")])
        ]
    );
    api.push(said(3, 52, (ADMIN, "Shorin"), "@miyu_test_bot 再来", true));
    let second = api
        .until(|method, params| {
            method == "sendMessage"
                && params["text"]
                    .as_str()
                    .is_some_and(|text| text.ends_with("没人"))
        })
        .await;
    assert_ne!(second["reply_parameters"]["message_id"], 999, "{second}");
    assert!(second.get("entities").is_none(), "{second}");
    let found = results(&home, 2).await;
    assert_eq!(
        found[2..],
        [
            tg_sentence("not-found", &[]),
            tg_sentence("not-members", &[("who", "30003")])
        ]
    );
    api.push(said(4, 53, (ADMIN, "Shorin"), "@miyu_test_bot 查查", true));
    api.until(|method, params| {
        method == "sendMessage"
            && params["text"]
                .as_str()
                .is_some_and(|text| text.ends_with("查到了"))
    })
    .await;
    let found = results(&home, 3).await;
    let history = found.last().expect("查了记录");
    assert!(
        history.contains("Alice (20001) msg=50: 今晚吃火锅"),
        "{history}"
    );
    // 群里的工具面只有 Telegram 的（核心照名字的前缀放，IM-3）：说话、记录这几件，没有 QQ 的。
    let (_, request) = script.requests().into_iter().last().expect("请求过模型");
    let names: Vec<String> = request.tools.iter().map(|tool| tool.name.clone()).collect();
    for wanted in [
        "tg_skip_reply",
        "tg_send",
        "tg_quote",
        "tg_mention",
        "tg_chat_history",
        "tg_activity_ranking",
    ] {
        assert!(
            names.iter().any(|name| name == wanted),
            "{wanted}：{names:?}"
        );
    }
    assert!(
        !names.iter().any(|name| name.starts_with("qq_")),
        "{names:?}"
    );
    home.stop_extensions().await;
}
