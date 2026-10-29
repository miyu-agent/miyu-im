//! 禁言（施工 O-50，`onebot.md` 第一条「禁言」，2026-10-17 项目主人定）：真核心拉起真桥、假 NapCat。禁言那一刻、解禁那一刻各记一块
//! 事实（谁禁的、多久；怎么解的、谁解的）；禁言以后 `mute_window_seconds` 以内判了要回的排着，更晚的只记下；解禁以后同一个人的
//! 并成一条，最多回 `mute_replies` 条，别人的照常算进限流，满了的、多的不出声地丢掉。

use std::sync::Arc;

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::*;

/// 群号。
const GROUP: i64 = 668;

/// 群主（禁她、解她的那一个；`group_ban` 的 `operator_id`）。
const OWNER: i64 = 40004;

/// 群里的别人。
const LIN: i64 = 20002;
const WANG: i64 = 20003;

/// 假 NapCat 认得的群成员。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (OWNER, "群主", "zhu")];

/// 群里不抽样（别人的话只记下）。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0 }\n";

/// 场所编号。
fn venue() -> String {
    format!("qq:group:{GROUP}")
}

/// `who` 在群里 @ 她：第 `message` 条。各人说的不一样，不成复读（施工 I-28）。
fn calls(napcat: &Answering, who: i64, message: i64) {
    let words = json!([at(BOT), plain(&format!(" 在吗，我是{who}"))]);
    napcat.send(group_frame(GROUP, who, message, words, ("某人", "x")));
}

/// 等到第 `message` 条的判断记下：交回它的 `body`。
async fn decided(home: &Home, message: i64) -> Value {
    let cause = format!("qq:{BOT}:{message}:{TIME}/decided");
    let events = until_event(&home.root, &venue(), |event| event["cause"] == cause).await;
    events
        .iter()
        .find(|event| event["cause"] == cause)
        .map(|event| event["body"].clone())
        .expect("记了")
}

/// 群会话里桥记的事实（`context.injected`）里 `kind` 的那几块的字。
fn facts(events: &[Value], kind: &str) -> Vec<String> {
    of_kind(events, "context.injected")
        .iter()
        .filter(|event| event["body"]["kind"] == kind)
        .filter_map(|event| {
            event["body"]["text"]
                .as_str()
                .map(|text| text.trim_end().to_string())
        })
        .collect()
}

/// 出厂的事实模板 `name`，字段照 `fields` 换，去掉行尾。
fn fact(name: &str, fields: &[(&str, &str)]) -> String {
    let path = resources().join(format!("packages/miyu-im/data/facts/{name}.txt"));
    let mut text = std::fs::read_to_string(path).expect("读得出");
    for (key, value) in fields {
        text = text.replace(&format!("{{{key}}}"), value);
    }
    text.trim_end().to_string()
}

/// 群里第 `message` 条消息记下的序号（`message.user` 的 `cause` 是它的命令编号）。
fn seq_of(events: &[Value], message: i64) -> u64 {
    let cause = format!("qq:{BOT}:{message}:{TIME}");
    events
        .iter()
        .find(|event| event["kind"] == "message.user" && event["cause"] == cause)
        .and_then(|event| event["seq"].as_u64())
        .expect("记了")
}

/// 等到群会话里有 `n` 条 `kind` 的事件：交回那时的全部事件。
async fn until_count(home: &Home, kind: &str, n: usize) -> Vec<Value> {
    until_events(&home.root, &venue(), |events| {
        of_kind(events, kind).len() == n
    })
    .await
}

/// 等到运行日志里有 `wanted`。
async fn until_log(home: &Home, wanted: &str) {
    let deadline = tokio::time::Instant::now() + WAIT;
    while !run_log(&home.root).contains(wanted) {
        assert!(
            tokio::time::Instant::now() < deadline,
            "运行日志里等不到 {wanted}：{}",
            run_log(&home.root)
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

/// 主线开过的回合各自的触发（`turn.started` 的 `triggers`），照先后。
fn triggers(events: &[Value]) -> Vec<Value> {
    of_kind(events, "turn.started")
        .iter()
        .map(|event| event["body"]["triggers"].clone())
        .collect()
}

#[tokio::test]
async fn the_mute_and_the_lift_are_noted_with_who_and_how_long() {
    let (home, napcat, _) = started(&Script::new([]), RULES, "", MEMBERS).await;
    // 群主先说过话：名字在缓存里。
    napcat.send(group_frame(
        GROUP,
        OWNER,
        1,
        json!([plain("开会")]),
        ("群主", "zhu"),
    ));
    decided(&home, 1).await;
    napcat.send(group_ban(GROUP, BOT, "ban", Some(5400)));
    let events = until_events(&home.root, &venue(), |events| {
        !facts(events, "muted").is_empty()
    })
    .await;
    assert_eq!(
        facts(&events, "muted"),
        [fact("muted", &[("by", "群主"), ("duration", "1h 30m")])]
    );
    napcat.send(group_ban(GROUP, BOT, "lift_ban", Some(0)));
    let events = until_events(&home.root, &venue(), |events| {
        !facts(events, "unmuted").is_empty()
    })
    .await;
    assert_eq!(
        facts(&events, "unmuted"),
        [fact("unmuted", &[("why", "lifted"), ("by", "群主")])]
    );
    // 再来一次解禁的通知（重了）：没禁着，不再记。
    napcat.send(group_ban(GROUP, BOT, "lift_ban", Some(0)));
    until_log(&home, "unmute ignored").await;
    let events = venue_events(&home.root, &venue());
    assert_eq!(of_kind(&events, "ext.im.venues.unmuted").len(), 1);
    assert_eq!(facts(&events, "unmuted").len(), 1);
    stopped(home).await;
}

#[tokio::test]
async fn deferred_calls_merge_per_person_and_only_so_many_are_answered() {
    let script = Script::new([Play::Says("在")]);
    let tuned = json!({"mute_replies": 1});
    let (home, mut napcat, _) = started_tuned(Arc::new(script.clone()), RULES, &tuned, |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    napcat.send(group_ban(GROUP, BOT, "ban", Some(600)));
    until_count(&home, "ext.im.venues.muted", 1).await;
    // 终端管理员两次、小林一次：都排着。
    calls(&napcat, ADMIN, 1);
    calls(&napcat, LIN, 2);
    calls(&napcat, ADMIN, 3);
    for message in 1..=3 {
        assert_eq!(
            decided(&home, message).await["outcome"],
            "deferred",
            "第 {message} 条"
        );
    }
    napcat.send(group_ban(GROUP, BOT, "lift_ban", Some(0)));
    assert_eq!(napcat.group_reply(GROUP).await, "在");
    let events = until_count(&home, "turn.ended", 1).await;
    // 终端管理员的两条并成一轮；最多回 1 条，小林的丢掉。
    let seqs: Vec<u64> = [1, 3]
        .iter()
        .map(|message| seq_of(&events, *message))
        .collect();
    assert_eq!(triggers(&events), [json!(seqs)]);
    until_log(&home, "deferred reply dropped").await;
    let log = run_log(&home.root);
    assert!(log.contains("why=too_many"), "{log}");
    assert!(napcat.pending().is_none(), "小林的不回、不出声");
    stopped(home).await;
}

#[tokio::test]
async fn calls_after_the_window_are_only_recorded() {
    let tuned = json!({"mute_window_seconds": 1});
    let (home, napcat, _) = started_tuned(Arc::new(Script::new([])), RULES, &tuned, |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    napcat.send(group_ban(GROUP, BOT, "ban", Some(600)));
    until_count(&home, "ext.im.venues.muted", 1).await;
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    calls(&napcat, ADMIN, 1);
    let body = decided(&home, 1).await;
    assert_eq!(
        (&body["inbound"], &body["why"], &body["outcome"]),
        (&json!("record_only"), &json!("muted"), &json!("record")),
        "过了窗口：暂时离开了群聊，只记下"
    );
    stopped(home).await;
}

#[tokio::test]
async fn the_rate_limit_counts_when_deferred_calls_are_answered() {
    // 一小时一轮：小林的那一轮开了，老王的就满了，丢掉；终端管理员不受限流，照回。
    let rules = format!(
        "{RULES}\n[[rule]]\nmatch = {{ kind = \"group\", group = [{GROUP}] }}\nrate = \"1/1h\"\n"
    );
    let script = Script::new([Play::Says("小林好"), Play::Says("老板好")]);
    let (home, mut napcat, _) = started(&script, &rules, "", MEMBERS).await;
    napcat.send(group_ban(GROUP, BOT, "ban", Some(600)));
    until_count(&home, "ext.im.venues.muted", 1).await;
    calls(&napcat, LIN, 1);
    calls(&napcat, WANG, 2);
    calls(&napcat, ADMIN, 3);
    for message in 1..=3 {
        assert_eq!(
            decided(&home, message).await["outcome"],
            "deferred",
            "第 {message} 条"
        );
    }
    napcat.send(group_ban(GROUP, BOT, "lift_ban", Some(0)));
    let mut heard = Vec::new();
    for _ in 0..2 {
        let message = napcat.group_message(GROUP).await;
        let words: String = message
            .iter()
            .filter_map(|segment| segment["data"]["text"].as_str())
            .collect();
        heard.push(words);
    }
    heard.sort();
    assert_eq!(heard, ["小林好", "老板好"]);
    until_log(&home, "why=rate_limited").await;
    let events = venue_events(&home.root, &venue());
    let opened: Vec<Value> = triggers(&events);
    assert!(
        !opened.contains(&json!([seq_of(&events, 2)])),
        "老王的没开轮：{opened:?}"
    );
    assert!(napcat.pending().is_none());
    stopped(home).await;
}

#[tokio::test]
async fn a_ban_that_runs_out_is_noted_even_with_nothing_waiting() {
    // 什么都没排着：照投影里禁到的时刻醒了补记（QQ 到点不推通知）。
    let (home, napcat, _) = started(&Script::new([]), RULES, "", MEMBERS).await;
    napcat.send(group_ban(GROUP, BOT, "ban", Some(1)));
    let events = until_events(&home.root, &venue(), |events| {
        !facts(events, "unmuted").is_empty()
    })
    .await;
    assert_eq!(
        facts(&events, "unmuted"),
        [fact("unmuted", &[("why", "expired"), ("by", "")])]
    );
    assert_eq!(
        of_kind(&events, "ext.im.venues.unmuted")[0]["body"],
        json!({"why": "expired"})
    );
    stopped(home).await;
}
