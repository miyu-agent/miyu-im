//! 时不时看一眼（施工 I-89，设计 05）：真核心照开关拉起真桥、假 NapCat，`bridge.json` 的 `glance_seconds` 改成 2 秒，经网页的样子存卡片。
//! 名单里的群卡打开看一眼：核心的定时到点，上次以后有没人叫她的新消息的开一轮（带 `glance` 事实），她照剧本说的发回群里；那一轮
//! 结束了照间隔再排；没有新消息的不开、照间隔再排；卡片关了删掉定时，再来新消息也不看。睡着的不开，顺延到醒。私聊（I-89 下）到点
//! 不看有没有新消息，开一轮只带事实；她用 `qq_glance` 定了下一次的照她的，那一轮结束不再照出厂的间隔排。

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::*;

/// 群里的别人。
const LIN: i64 = 20005;

/// 名单里的群。
const GROUP: i64 = 778;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 系统的场所规则：只有一条配群的空规则。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\n";

/// 两个间隔多一点：照间隔再排的那一次到过点了。
const TWO_GLANCES: Duration = Duration::from_secs(5);

/// 小林在群里随口说的一句（没叫她），平台编号是 `message`。
fn chat(message: i64, text: &str) -> Value {
    group_frame(GROUP, LIN, message, json!([plain(text)]), ("小林", "lin"))
}

/// 存卡片：这个群的卡只写看一眼开没开。
async fn glance(head: &mut Head, on: bool) {
    let cards = json!({"groups": [{"id": GROUP, "set": {"glance": on}}]});
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
}

/// 这个群的会话开过几轮。
fn turns(home: &Home, venue: &str) -> usize {
    of_kind(&venue_events(&home.root, venue), "turn.started").len()
}

#[tokio::test]
async fn a_glance_opens_on_new_messages_only_and_stops_when_turned_off() {
    let script = Script::new([Play::Says("在看呢"), Play::Says("又来了")]);
    let tuned = json!({"glance_seconds": 2});
    let (home, mut napcat, _) = started_tuned(Arc::new(script.clone()), RULES, &tuned, |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let venue = format!("qq:{BOT}:group:{GROUP}");
    // 1：没人叫她：只记下，群会话有了。
    napcat.send(chat(1, "今天吃什么"));
    until_event(&home.root, &venue, |event| {
        event["kind"] == "ext.im.chat.decided"
    })
    .await;
    // 打开看一眼：存了当场排上定时，从这一刻起来的才算新的。
    glance(&mut head, true).await;
    napcat.send(chat(2, "有人吗"));
    assert_eq!(napcat.group_reply(GROUP).await, "在看呢");
    let events = until_events(&home.root, &venue, |events| {
        of_kind(events, "turn.ended").len() == 1
    })
    .await;
    let opened = of_kind(&events, "turn.started");
    assert_eq!(opened.len(), 1, "{opened:#?}");
    assert!(
        events
            .iter()
            .any(|event| event.to_string().contains("<glance>")),
        "带一块看一眼的事实：{events:#?}"
    );
    // 再到点没有新消息：不开，照间隔再排。
    tokio::time::sleep(TWO_GLANCES).await;
    assert_eq!(turns(&home, &venue), 1, "没有新消息不请求模型");
    let log = run_log(&home.root);
    assert!(log.matches("glance armed").count() >= 3, "{log}");
    // 3：又来一句：下一次到点开第二轮。
    napcat.send(chat(3, "人呢"));
    assert_eq!(napcat.group_reply(GROUP).await, "又来了");
    until_events(&home.root, &venue, |events| {
        of_kind(events, "turn.ended").len() == 2
    })
    .await;
    // 关掉：对账删掉定时，再来新消息也不看。
    glance(&mut head, false).await;
    let log = until_log(&home, "cancelled=1").await;
    assert!(log.contains("glances reconciled"), "{log}");
    napcat.send(chat(4, "走了"));
    tokio::time::sleep(TWO_GLANCES).await;
    assert_eq!(turns(&home, &venue), 2, "关了不再看");
    assert!(napcat.pending().is_none(), "别的什么都不发");
    stopped(home).await;
}

#[tokio::test]
async fn a_glance_while_asleep_waits_for_the_wake() {
    let script = Script::new([]);
    let tuned = json!({"glance_seconds": 2});
    let (home, mut napcat, _) =
        started_tuned(Arc::new(script.clone()), &sleepy(), &tuned, |napcat| {
            napcat.answering(MEMBERS)
        })
        .await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let venue = format!("qq:{BOT}:group:{GROUP}");
    napcat.send(chat(1, "今天吃什么"));
    until_event(&home.root, &venue, |event| {
        event["kind"] == "ext.im.chat.decided"
    })
    .await;
    glance(&mut head, true).await;
    napcat.send(chat(2, "有人吗"));
    // 到点睡着：顺延到睡觉时段结束，第二个定时排在一个小时左右以后，不是两秒以后。
    let log = until_log(&home, "schedule=t2").await;
    assert!(log.contains("glance after sleep"), "{log}");
    tokio::time::sleep(TWO_GLANCES).await;
    assert!(!run_log(&home.root).contains("schedule=t3"), "{log}");
    assert_eq!(turns(&home, &venue), 0, "睡着的不开");
    assert!(napcat.pending().is_none(), "别的什么都不发");
    stopped(home).await;
}

/// 白名单成员，有私聊卡。
const FRIEND: i64 = 40015;

#[tokio::test]
async fn a_private_glance_opens_without_new_messages_and_her_tool_sets_the_next() {
    let script = Script::new([
        Play::Says("在呢"),
        Play::calls(&[("qq_glance", r#"{"minutes": 60}"#)]),
        Play::Says("想你了"),
    ]);
    let tuned = json!({"glance_seconds": 2});
    let onebot = format!("whitelist = [\"qq:{FRIEND}\"]\n");
    let (home, mut napcat, _) = started_tuned_with(
        Arc::new(script.clone()),
        (RULES, &onebot),
        &tuned,
        |napcat| napcat.answering(MEMBERS),
    )
    .await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let venue = format!("qq:{BOT}:private:{FRIEND}");
    // 1：私聊里说一句，她回了：会话有了。
    napcat.send(private_frame(FRIEND, 1, json!([plain("早")])));
    assert_eq!(private_reply(&mut napcat).await, "在呢");
    until_events(&home.root, &venue, |events| {
        of_kind(events, "turn.ended").len() == 1
    })
    .await;
    // 打开看一眼：到点没有新消息也开一轮，她定下一次 60 分钟以后，再说一句。
    let cards = json!({"privates": [{"id": FRIEND, "set": {"glance": true}}]});
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    assert_eq!(private_reply(&mut napcat).await, "想你了");
    let events = until_events(&home.root, &venue, |events| {
        of_kind(events, "turn.ended").len() == 2
    })
    .await;
    let text = format!("{events:?}");
    assert!(text.contains("<glance>"), "带一块看一眼的事实：{events:#?}");
    assert!(
        text.contains("Next look in 60 min."),
        "工具答了：{events:#?}"
    );
    // 那一轮结束不再照两秒排：过两个间隔也没有第三轮。
    tokio::time::sleep(TWO_GLANCES).await;
    assert_eq!(turns(&home, &venue), 2, "照她定的 60 分钟");
    let log = run_log(&home.root);
    assert_eq!(log.matches("glance armed").count(), 2, "{log}");
    assert!(napcat.pending().is_none(), "别的什么都不发");
    stopped(home).await;
}

/// 下一个动作是发给 [`FRIEND`] 的 `send_private_msg`：交回里面的字。
async fn private_reply(napcat: &mut Answering) -> String {
    let action = napcat.action().await;
    assert_eq!(action["action"], "send_private_msg", "{action}");
    assert_eq!(action["params"]["user_id"], FRIEND, "{action}");
    action["params"]["message"][0]["data"]["text"]
        .as_str()
        .expect("有字")
        .to_string()
}

/// 系统的场所规则：群的睡觉时间盖住此刻（前后各一个小时，照本机的时区；同 `whitelist.rs`）。
fn sleepy() -> String {
    let now = jiff::Zoned::now();
    let minute = i64::from(now.hour()) * 60 + i64::from(now.minute());
    let clock = |minute: i64| {
        let minute = minute.rem_euclid(24 * 60);
        format!("{:02}:{:02}", minute / 60, minute % 60)
    };
    format!(
        "[[rule]]\nmatch = {{ kind = \"group\" }}\nsleep = \"{}-{}\"\n",
        clock(minute - 60),
        clock(minute + 60),
    )
}

/// 等到运行日志里有 `wanted`：交回那时的运行日志。
async fn until_log(home: &Home, wanted: &str) -> String {
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        let log = run_log(&home.root);
        if log.contains(wanted) {
            return log;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "等不到 {wanted}：\n{log}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
