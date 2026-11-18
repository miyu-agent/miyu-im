//! 补拉消息（施工 I-15，设计 18 第十一节）：真核心拉起真桥、假 NapCat。群里实时收到一条以后断开，再连上时桥照记着的问
//! `get_group_msg_history`，漏掉的记进这个群的场所会话：带 `backfill`、`ambient`、`sent`，不开回合；实时收到过的那一条核心只
//! 记一次，更早的、她自己发的不补。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::group::*;
use crate::support::platform::*;
use crate::support::*;

/// 群里的别人。
const LIN: i64 = 20004;

/// 假 NapCat 认得的群成员。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// `get_group_msg_history` 交回的一条：`user` 在 `time` 发的、编号是 `id` 的、字是 `text`（照 NapCat 的样子带 `post_type`）。
fn history(user: i64, id: i64, time: i64, text: &str) -> Value {
    let mut frame = group_frame(GROUP, user, id, json!([plain(text)]), ("小林", "lin"));
    frame["time"] = json!(time);
    frame
}

#[tokio::test(flavor = "multi_thread")]
async fn messages_missed_while_away_are_backfilled_into_the_chat_log_only() {
    let (home, napcat, listen) = started(&Script::new([]), "", "", MEMBERS).await;
    let venue = format!("qq:30003:group:{GROUP}");
    napcat.send(group_frame(
        GROUP,
        LIN,
        1,
        json!([plain("先说一句")]),
        ("小林", "lin"),
    ));
    until_event(&home.root, &venue, |event| event["kind"] == "message.user").await;
    drop(napcat);
    until_logged(&home.root, |log| log.contains("napcat disconnected")).await;

    let mut again = admin_napcat(listen).await;
    let asked = again
        .history(json!([
            history(LIN, 0, TIME - 60, "更早的"),
            history(LIN, 1, TIME, "先说一句"),
            history(LIN, 2, TIME + 60, "断着时说的"),
            history(BOT, 3, TIME + 90, "她自己的"),
            history(LIN, 4, TIME + 120, "又一句"),
        ]))
        .await;
    assert_eq!(asked["group_id"], GROUP, "{asked}");
    let _serving = again.answering(MEMBERS);
    let events = until_events(&home.root, &venue, |events| said(events).len() >= 3).await;
    let said = said(&events);
    let texts: Vec<&str> = said
        .iter()
        .filter_map(|event| event["body"]["blocks"][0]["text"].as_str())
        .collect();
    assert_eq!(texts, ["先说一句", "断着时说的", "又一句"], "{said:#?}");
    let live = &said[0]["body"]["venue"];
    assert!(
        live["backfill"].is_null() && live["sent"].is_null(),
        "{live}"
    );
    for (event, at) in said[1..].iter().zip([TIME + 60, TIME + 120]) {
        let fields = &event["body"]["venue"];
        assert_eq!(
            (&fields["backfill"], &fields["ambient"]),
            (&json!(true), &json!(true)),
            "{event}"
        );
        let sent = jiff::Timestamp::from_second(at).expect("合法的时刻");
        assert_eq!(
            fields["sent"],
            format!("{}", sent.strftime("%Y-%m-%dT%H:%M:%S.000Z")),
            "{event}"
        );
    }
    assert!(
        of_kind(&events, "turn.started").is_empty(),
        "补回来的不开回合：{events:#?}"
    );
    let log = until_logged(&home.root, |log| {
        log.matches("message backfilled").count() >= 2
    })
    .await;
    assert!(!log.contains("backfill truncated"), "{log}");
    stopped(home).await;
}

/// 断得久，一页装不下（施工 I-254，2026-11-19 项目主人定：原来只补最近 30 条）：照第一页最早那一条的编号往前再问一页，两页都补进来；
/// 第二页够到了记着的那一刻就不再问。
#[tokio::test(flavor = "multi_thread")]
async fn a_long_absence_is_backfilled_page_by_page() {
    let (home, napcat, listen) = started(&Script::new([]), "", "", MEMBERS).await;
    let venue = format!("qq:30003:group:{GROUP}");
    napcat.send(group_frame(
        GROUP,
        LIN,
        1,
        json!([plain("先说一句")]),
        ("小林", "lin"),
    ));
    until_event(&home.root, &venue, |event| event["kind"] == "message.user").await;
    drop(napcat);
    until_logged(&home.root, |log| log.contains("napcat disconnected")).await;

    let mut again = admin_napcat(listen).await;
    // 第一页：最近的 30 条，都晚于记着的那一刻。
    let newest: Vec<Value> = (0..30)
        .map(|n| history(LIN, 100 + n, TIME + 1000 + n, &format!("后{n}")))
        .collect();
    let first = again.history(Value::Array(newest)).await;
    assert!(
        first.get("message_seq").is_none(),
        "第一页从最新的问：{first}"
    );
    // 第二页：照第一页最早那一条（编号 100）往前，够到记着的那一刻。
    let older: Vec<Value> = std::iter::once(history(LIN, 1, TIME, "先说一句"))
        .chain((0..10).map(|n| history(LIN, 10 + n, TIME + 100 + n, &format!("前{n}"))))
        .collect();
    let second = again.history(Value::Array(older)).await;
    assert_eq!(second["message_seq"], 100, "{second}");
    let _serving = again.answering(MEMBERS);
    let events = until_events(&home.root, &venue, |events| said(events).len() >= 41).await;
    let texts: Vec<String> = said(&events)
        .iter()
        .filter_map(|event| {
            event["body"]["blocks"][0]["text"]
                .as_str()
                .map(str::to_string)
        })
        .collect();
    assert_eq!(
        texts.len(),
        41,
        "实时的一条、第二页十条、第一页三十条：{texts:?}"
    );
    assert_eq!(
        (texts[1].as_str(), texts[40].as_str()),
        ("前0", "后29"),
        "照先后"
    );
    stopped(home).await;
}
