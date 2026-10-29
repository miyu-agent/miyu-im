//! 进站挂接点（施工 O-43 中，`onebot.md` 第一条「挂接点」第 3 条，`extending.md` 第二条）：真核心照开关拉起真桥，管理员家目录里
//! 另装导出 `chat.inbound` 的测试包。
//!
//! - 回 `record`：别人 @ 她只记下，判断写 `hook`；终端管理员 @ 她照回、不写 `hook`。问的是这一条的场所、号、发的人、正文。
//! - 回 `reply`：别人随口说的（不 @、不抽样）也直接回，不问判官；两个包一个 `record` 一个 `reply` 照 `record`。
//! - 不回的：等 `hook_millis`（出厂 1.5 秒）以后照常，运行日志记一行。

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};
use miyu_store::root::DataRoot;

use crate::support::group::*;
use crate::support::hooked::{install, until_hooked};
use crate::support::*;

/// 群里的别人。
const LIN: i64 = 20004;

/// 群号。
const GROUP: i64 = 777;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 系统的场所规则：群都不抽样。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0 }\n";

/// 平台编号是 `message` 的那一条判下来的样子。
fn decided(events: &[Value], message: i64) -> Option<Value> {
    let cause = format!("qq:{BOT}:{message}:{TIME}/decided");
    events
        .iter()
        .find(|event| event["kind"] == "ext.im.chat.decided" && event["cause"] == cause)
        .map(|event| event["body"].clone())
}

/// `user` 在群里说第 `message` 条：`at` 的 @ 她。
fn says(user: i64, message: i64, at_her: bool) -> Value {
    let mut words = Vec::new();
    if at_her {
        words.push(at(BOT));
    }
    words.push(plain(" 在吗"));
    group_frame(GROUP, user, message, json!(words), ("小林", "lin"))
}

/// 数据根里测试包 `id` 记调用的文件。
fn log_of(root: &DataRoot, id: &str) -> std::path::PathBuf {
    root.path().join(format!("{id}.calls"))
}

/// 测试包 `id`：进站挂接点照 `answer` 答（`answer:` 的 JSON，或者 `slow`）。
fn hook(root: &DataRoot, id: &str, answer: &str) {
    let record = format!("record:{}", log_of(root, id).display());
    let how = match answer {
        "slow" => "slow:chat.inbound".to_string(),
        answer => format!("answer:chat.inbound:{answer}"),
    };
    install(root, id, &[record, how], &["chat.inbound"]);
}

/// 测试包 `id` 记下的调用。
fn calls(root: &DataRoot, id: &str) -> Vec<Value> {
    std::fs::read_to_string(log_of(root, id))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[tokio::test]
async fn record_holds_her_back_except_for_the_admin() {
    let script = Script::new([Play::Says("在")]);
    let (home, mut napcat, _) = started_installed(&script, RULES, MEMBERS, |root| {
        hook(root, "guard", r#"{"route":"record"}"#)
    })
    .await;
    until_hooked(&home, 1).await;
    let venue = format!("qq:30003:group:{GROUP}");
    napcat.send(says(LIN, 1, true));
    let events = until_events(&home.root, &venue, |events| decided(events, 1).is_some()).await;
    let held = decided(&events, 1).expect("记了");
    assert_eq!(
        (&held["outcome"], &held["hook"]),
        (
            &json!("record"),
            &json!({"package": "guard", "route": "record"})
        ),
        "{held}"
    );
    let asked = &calls(&home.root, "guard")[0];
    assert_eq!(asked["method"], "chat.inbound");
    assert_eq!(asked["from"], "miyu-im");
    assert_eq!(
        asked["params"],
        json!({"platform": "qq", "venue": venue, "bot": BOT, "msg": "1", "sender": {"id": format!("qq:{LIN}"), "name": "小林"}, "text": "@米尤 在吗", "mentions_me": true, "quotes_me": false, "media": []}),
    );
    // 终端管理员的照回：挂接点也问了，意见不算。
    napcat.send(says(ADMIN, 2, true));
    assert_eq!(napcat.group_reply(GROUP).await, "在");
    let events = until_events(&home.root, &venue, |events| decided(events, 2).is_some()).await;
    let admin = decided(&events, 2).expect("记了");
    assert_eq!(admin["outcome"], "reply", "{admin}");
    assert!(admin.get("hook").is_none(), "{admin}");
    assert!(napcat.pending().is_none(), "别的什么都不发");
    stopped(home).await;
}

#[tokio::test]
async fn reply_answers_even_a_plain_message_without_the_judge() {
    let script = Script::new([Play::Says("在的"), Play::Says("还在")]);
    let (home, mut napcat, _) = started_installed(&script, RULES, MEMBERS, |root| {
        hook(root, "aaa", r#"{"route":"reply"}"#);
        hook(root, "zzz", r#"{"route":"pass"}"#);
    })
    .await;
    until_hooked(&home, 2).await;
    let venue = format!("qq:30003:group:{GROUP}");
    napcat.send(says(LIN, 1, false));
    assert_eq!(napcat.group_reply(GROUP).await, "在的");
    let events = until_events(&home.root, &venue, |events| decided(events, 1).is_some()).await;
    let replied = decided(&events, 1).expect("记了");
    assert_eq!(
        (&replied["outcome"], &replied["hook"]),
        (
            &json!("reply"),
            &json!({"package": "aaa", "route": "reply"})
        ),
        "{replied}"
    );
    assert!(replied.get("judge").is_none(), "不问判官：{replied}");
    assert_eq!(calls(&home.root, "zzz").len(), 1, "两个都问了");
    // 本来就回的（终端管理员 @ 她）不算改了，判断不写 `hook`。
    napcat.send(says(ADMIN, 2, true));
    assert_eq!(napcat.group_reply(GROUP).await, "还在");
    let events = until_events(&home.root, &venue, |events| decided(events, 2).is_some()).await;
    let admin = decided(&events, 2).expect("记了");
    assert!(admin.get("hook").is_none(), "{admin}");
    stopped(home).await;
}

#[tokio::test]
async fn record_beats_reply() {
    let script = Script::new([]);
    let (home, napcat, _) = started_installed(&script, RULES, MEMBERS, |root| {
        hook(root, "aaa", r#"{"route":"reply"}"#);
        hook(root, "bbb", r#"{"route":"record"}"#);
    })
    .await;
    until_hooked(&home, 2).await;
    let venue = format!("qq:30003:group:{GROUP}");
    napcat.send(says(LIN, 1, true));
    let events = until_events(&home.root, &venue, |events| decided(events, 1).is_some()).await;
    let held = decided(&events, 1).expect("记了");
    assert_eq!(
        (&held["outcome"], &held["hook"]),
        (
            &json!("record"),
            &json!({"package": "bbb", "route": "record"})
        ),
        "{held}"
    );
    stopped(home).await;
}

#[tokio::test]
async fn a_silent_hook_is_waited_for_then_ignored() {
    let script = Script::new([Play::Says("来了")]);
    let (home, mut napcat, _) =
        started_installed(&script, RULES, MEMBERS, |root| hook(root, "slow", "slow")).await;
    until_hooked(&home, 1).await;
    let venue = format!("qq:30003:group:{GROUP}");
    let sent = std::time::Instant::now();
    napcat.send(says(LIN, 1, true));
    // 判官没剧本（一次性入口拒了），冲她来的照回（O-37）：等过挂接点才轮到它。
    assert_eq!(napcat.group_reply(GROUP).await, "来了");
    assert!(
        sent.elapsed() >= std::time::Duration::from_millis(1500),
        "等满了 hook_millis"
    );
    let events = until_events(&home.root, &venue, |events| decided(events, 1).is_some()).await;
    assert!(decided(&events, 1).expect("记了").get("hook").is_none());
    let log = run_log(&home.root);
    assert!(
        log.lines().any(|line| line.contains("hook failed")
            && line.contains("package=slow")
            && line.contains("why=timeout")),
        "{log}"
    );
    stopped(home).await;
}

#[tokio::test]
async fn an_answer_it_cannot_read_is_a_pass() {
    let script = Script::new([]);
    let (home, napcat, _) = started_installed(&script, RULES, MEMBERS, |root| {
        hook(root, "odd", r#"{"route":"maybe"}"#);
    })
    .await;
    until_hooked(&home, 1).await;
    let venue = format!("qq:30003:group:{GROUP}");
    // 别人随口说的：照常只记下（不抽样）；回得不对的不当成 `reply`。
    napcat.send(says(LIN, 1, false));
    let events = until_events(&home.root, &venue, |events| decided(events, 1).is_some()).await;
    let plain = decided(&events, 1).expect("记了");
    assert_eq!(plain["outcome"], "record", "{plain}");
    assert!(plain.get("hook").is_none(), "{plain}");
    let log = run_log(&home.root);
    assert!(
        log.lines().any(|line| line.contains("hook failed")
            && line.contains("package=odd")
            && line.contains("why=bad_answer")),
        "{log}"
    );
    stopped(home).await;
}

#[tokio::test]
async fn reply_does_not_get_past_the_rate_limit() {
    // 一小时一个回合：头一条照 `reply` 回了，第二条额度满了，`reply` 不越过它，只记下。
    let script = Script::new([Play::Says("在的")]);
    let rules = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0 }\nrate = \"1/3600s\"\n";
    let (home, mut napcat, _) = started_installed(&script, rules, MEMBERS, |root| {
        hook(root, "eager", r#"{"route":"reply"}"#);
    })
    .await;
    until_hooked(&home, 1).await;
    let venue = format!("qq:30003:group:{GROUP}");
    napcat.send(says(LIN, 1, false));
    assert_eq!(napcat.group_reply(GROUP).await, "在的");
    until_events(&home.root, &venue, |events| {
        of_kind(events, "turn.ended").len() == 1
    })
    .await;
    napcat.send(says(LIN, 2, false));
    let events = until_events(&home.root, &venue, |events| decided(events, 2).is_some()).await;
    let held = decided(&events, 2).expect("记了");
    assert_eq!(held["outcome"], "record", "{held}");
    assert!(held.get("hook").is_none(), "{held}");
    assert!(napcat.pending().is_none(), "别的什么都不发");
    stopped(home).await;
}

#[tokio::test]
async fn the_inbound_hook_hears_what_a_message_carries() {
    // 施工 O-52：带的东西照先后交给挂接点：第几样、种类、平台的编号、名字、字节数（表情包包偷图照它自己去取）。
    let script = Script::new([]);
    let (home, napcat, _) = started_installed(&script, RULES, MEMBERS, |root| {
        hook(root, "guard", r#"{"route":"pass"}"#)
    })
    .await;
    until_hooked(&home, 1).await;
    let venue = format!("qq:30003:group:{GROUP}");
    let words = json!([
        plain("看"),
        {"type": "image", "data": {"file": "cat.png", "file_size": "2048", "sub_type": 0}},
        {"type": "image", "data": {"file": "doge.gif", "sub_type": 1, "summary": "[动画表情]"}},
    ]);
    napcat.send(group_frame(GROUP, LIN, 1, words, ("小林", "lin")));
    until_events(&home.root, &venue, |events| decided(events, 1).is_some()).await;
    let asked = &calls(&home.root, "guard")[0];
    assert_eq!(
        asked["params"]["media"],
        json!([
            {"index": 1, "kind": "image", "id": "cat.png", "size": 2048},
            {"index": 2, "kind": "sticker", "id": "doge.gif"},
        ])
    );
    stopped(home).await;
}
