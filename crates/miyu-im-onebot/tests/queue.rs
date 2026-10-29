//! 出站队列（施工 O-25 中，`onebot.md` 第一条「出站队列」）：真核心拉起真桥、假 NapCat，她照剧本、台词说。群里她的每一段先记
//! `ext.im.venues.queued` 再发，日志里入队在送达前面；NapCat 回失败的记 `failed {why: rejected, detail}`；她被禁言（假
//! NapCat 推 `group_ban`）记 `muted {until, by}`，禁言以后不久终端管理员 @ 她排着（施工 O-50），禁言以前开的一轮她的话排着，解禁的
//! 通知来了照先后发、记 `unmuted {why: lifted, by}`，`until` 到了自己发、补记 `unmuted {why: expired}`；别人被禁言、全员禁言不认；
//! 去重照入队算（排着的也算）。要等过期、断开再连、桥重启、私聊的在 `queue_waits.rs`；禁言的事实、排着的怎么回在 `muted.rs`。

use std::sync::Arc;

use serde_json::{Value, json};
use tokio::sync::oneshot;

use miyu_kernel::time::Timestamp;
use miyu_session::testkit::{Play, Script};

use crate::support::answering::refused;
use crate::support::group::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 一段最多 4 个字的群：她的一句拆成几段。
const SPLIT: i64 = 666;

/// 出厂参数的群：去重照整句比。
const PLAIN: i64 = 667;

/// 群里的别人。
const LIN: i64 = 20002;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 系统的场所规则：群都不抽样（别人没 @ 她的话只记下）、谁 @ 她都直接回（施工 I-33：禁言以前先开一轮，不再以人的身份代判官开）；
/// [`SPLIT`] 一段最多 4 个字。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0, judge_called = false }\n\n\
    [[rule]]\nmatch = { kind = \"group\", group = [666] }\noutbound = { split_chars = 4 }\n";

/// 入队、失败、禁言、解禁记成的事件。
const QUEUED: &str = "ext.im.venues.queued";
const FAILED: &str = "ext.im.venues.failed";
const MUTED: &str = "ext.im.venues.muted";
const UNMUTED: &str = "ext.im.venues.unmuted";

/// 群号是 `group` 的场所编号。
fn venue(group: i64) -> String {
    format!("qq:30003:group:{group}")
}

/// 终端管理员在群 `group` 里 @ 她：第 `message` 条。
fn admin_calls(napcat: &Answering, group: i64, message: i64) {
    let words = json!([at(BOT), plain(" 在吗")]);
    napcat.send(group_frame(
        group,
        ADMIN,
        message,
        words,
        ("终端管理员", "o"),
    ));
}

/// 等到群 `group` 里有 `n` 条 `kind` 的事件：交回那时的全部事件。
async fn until_count(home: &Home, group: i64, kind: &str, n: usize) -> Vec<Value> {
    until_events(&home.root, &venue(group), |events| {
        of_kind(events, kind).len() == n
    })
    .await
}

/// 一段字。
fn words(text: &str) -> Value {
    json!({"type": "text", "data": {"text": text}})
}

/// 等到群 `group` 里第 `message` 条消息的判断记下了：交回它的 `body`。
async fn decided(home: &Home, group: i64, message: i64) -> Value {
    let cause = format!("qq:{BOT}:{message}:{TIME}/decided");
    let events = until_event(&home.root, &venue(group), |event| event["cause"] == cause).await;
    let found = events.iter().find(|event| event["cause"] == cause);
    found.expect("记了")["body"].clone()
}

#[tokio::test]
async fn each_piece_is_queued_before_it_is_delivered() {
    let script = Script::new([Play::Says("看到大家了")]);
    let (home, mut napcat, _) = started(&script, RULES, "", MEMBERS).await;
    admin_calls(&napcat, SPLIT, 1);
    assert_eq!(napcat.group_message(SPLIT).await, [words("看到大家")]);
    assert_eq!(napcat.group_message(SPLIT).await, [words("了")]);
    let events = until_count(&home, SPLIT, "venue.delivered", 2).await;
    let (session, _) = venue_session(&home.root, &venue(SPLIT)).expect("有会话");
    let turn = of_kind(&events, "turn.started")[0]["seq"].clone();
    let queued = of_kind(&events, QUEUED);
    let bodies: Vec<Value> = queued.iter().map(|one| one["body"].clone()).collect();
    assert_eq!(
        bodies,
        [
            json!({"kind": "reply", "text": "看到大家", "line": session, "turn": turn}),
            json!({"kind": "reply", "text": "了", "line": session, "turn": turn}),
        ]
    );
    let delivered = of_kind(&events, "venue.delivered");
    for (queued, delivered) in queued.iter().zip(&delivered) {
        assert_eq!(queued["body"]["text"], delivered["body"]["text"]);
        assert!(
            queued["seq"].as_u64() < delivered["seq"].as_u64(),
            "先入队、再送达：{queued} {delivered}"
        );
    }
    assert!(of_kind(&events, FAILED).is_empty());
    stopped(home).await;
}

#[tokio::test]
async fn a_refused_piece_is_failed_as_rejected() {
    let models = Arc::new(Script::new([Play::Says("在")]));
    let (home, mut napcat, _) = started_tuned(models, RULES, &Value::Null, |napcat| {
        napcat.refusing(MEMBERS)
    })
    .await;
    admin_calls(&napcat, PLAIN, 1);
    assert_eq!(napcat.group_message(PLAIN).await, [words("在")]);
    let events = until_count(&home, PLAIN, FAILED, 1).await;
    let queued = &of_kind(&events, QUEUED)[0];
    let detail: String = refused().trim().chars().take(200).collect();
    assert_eq!(
        of_kind(&events, FAILED)[0]["body"],
        json!({"queued": queued["seq"], "why": "rejected", "detail": detail}),
        "NapCat 说的去掉空白、截到 200 个字符"
    );
    assert!(of_kind(&events, "venue.delivered").is_empty());
    let log = run_log(&home.root);
    assert!(log.contains("reply not sent"), "{log}");
    assert!(log.contains("why=rejected"), "{log}");
    stopped(home).await;
}

/// 一条发出去的消息里的字（引用、@ 不算）。
fn said_words(message: Vec<Value>) -> String {
    message
        .iter()
        .filter(|segment| segment["type"] == "text")
        .filter_map(|segment| segment["data"]["text"].as_str())
        .collect()
}

/// 小林在群 `group` 里 @ 她：第 `message` 条（施工 I-33：禁言以前开一轮）。
fn lin_calls(napcat: &Answering, group: i64, message: i64) {
    napcat.send(group_frame(
        group,
        LIN,
        message,
        json!([at(BOT), plain(" 大家好")]),
        ("小林", "lin"),
    ));
}

#[tokio::test]
async fn while_muted_her_words_wait_and_a_call_waits_until_the_lift() {
    let (release, released) = oneshot::channel();
    let lines = Lines::new([
        Line::says("看到大家了").released_by(released),
        Line::says("嗯"),
        Line::says("好"),
    ]);
    let (home, mut napcat, _) = started_by(Arc::new(lines), RULES, MEMBERS).await;
    // 禁言以前小林 @ 她：开一轮，她还没开口。
    lin_calls(&napcat, SPLIT, 2);
    until_count(&home, SPLIT, "turn.started", 1).await;
    let before = Timestamp::from_unix_millis(now_millis()).expect("在范围里");
    napcat.send(group_ban(SPLIT, BOT, "ban", Some(600)));
    let events = until_count(&home, SPLIT, MUTED, 1).await;
    let body = &of_kind(&events, MUTED)[0]["body"];
    let until: Timestamp = serde_json::from_value(body["until"].clone()).expect("写法同事件的时刻");
    let ahead = until.unix_millis() - before.unix_millis();
    assert!(
        (600_000..660_000).contains(&ahead),
        "本机此刻加 600 秒：{ahead}"
    );
    assert_eq!(body["by"], 40004, "谁禁的（施工 O-50）");
    // 禁言以后 5 分钟以内终端管理员 @ 她：照常判，判了要回的排着（施工 O-50），不开轮。
    admin_calls(&napcat, SPLIT, 1);
    let body = decided(&home, SPLIT, 1).await;
    assert_eq!(
        (&body["inbound"], &body["outcome"]),
        (&json!("pass"), &json!("deferred")),
        "{body}"
    );
    // 禁言以前开的那一轮她说了：她的话排着。
    release.send(()).expect("她在等");
    let events = until_count(&home, SPLIT, QUEUED, 2).await;
    let log = run_log(&home.root);
    assert!(log.contains("reply waiting"), "{log}");
    assert!(log.contains("why=muted"), "{log}");
    assert_eq!(
        of_kind(&events, "turn.started").len(),
        1,
        "终端管理员那一条没开轮"
    );
    assert!(of_kind(&events, "venue.delivered").is_empty(), "还没发");
    // 解禁的通知来了：记 `unmuted {why: lifted, by}`，她排着的话照先后发，排着的那一条开一轮回。
    napcat.send(group_ban(SPLIT, BOT, "lift_ban", Some(0)));
    assert_eq!(napcat.group_message(SPLIT).await, [words("看到大家")]);
    assert_eq!(napcat.group_message(SPLIT).await, [words("了")]);
    assert_eq!(said_words(napcat.group_message(SPLIT).await), "嗯");
    let events = until_count(&home, SPLIT, "venue.delivered", 3).await;
    assert_eq!(
        of_kind(&events, UNMUTED)[0]["body"],
        json!({"why": "lifted", "by": 40004})
    );
    // 解禁以后终端管理员 @ 她照回。
    admin_calls(&napcat, SPLIT, 3);
    assert_eq!(napcat.group_message(SPLIT).await, [words("好")]);
    assert!(napcat.pending().is_none());
    stopped(home).await;
}

#[tokio::test]
async fn a_shorter_ban_ends_by_itself_and_is_noted() {
    let (release, released) = oneshot::channel();
    let lines = Lines::new([Line::says("在").released_by(released), Line::says("嗯")]);
    let (home, mut napcat, _) = started_by(Arc::new(lines), RULES, MEMBERS).await;
    lin_calls(&napcat, PLAIN, 2);
    until_count(&home, PLAIN, "turn.started", 1).await;
    napcat.send(group_ban(PLAIN, BOT, "ban", Some(600)));
    until_count(&home, PLAIN, MUTED, 1).await;
    admin_calls(&napcat, PLAIN, 1);
    assert_eq!(decided(&home, PLAIN, 1).await["outcome"], "deferred");
    release.send(()).expect("她在等");
    until_count(&home, PLAIN, QUEUED, 1).await;
    // 管理员把禁言改短成 1 秒（NapCat 再推一次 `ban`）：到了就发；QQ 到点不推解禁，桥补记 `unmuted {why: expired}`（施工 O-50），
    // 排着的那一条开一轮回。
    napcat.send(group_ban(PLAIN, BOT, "ban", Some(1)));
    assert_eq!(napcat.group_message(PLAIN).await, [words("在")]);
    // 前一段刚发出去、群里最后一条可能是她自己的：带不带引用看回执先到没有，只比字。
    assert_eq!(said_words(napcat.group_message(PLAIN).await), "嗯");
    let events = until_count(&home, PLAIN, "venue.delivered", 2).await;
    assert_eq!(of_kind(&events, MUTED).len(), 2);
    assert_eq!(
        of_kind(&events, UNMUTED)
            .iter()
            .map(|one| one["body"].clone())
            .collect::<Vec<_>>(),
        [json!({"why": "expired"})],
        "到期补记一次"
    );
    assert!(of_kind(&events, FAILED).is_empty());
    stopped(home).await;
}

#[tokio::test]
async fn someone_else_or_everyone_banned_is_not_her() {
    let script = Script::new([Play::Says("在")]);
    let (home, mut napcat, _) = started(&script, RULES, "", MEMBERS).await;
    napcat.send(group_ban(PLAIN, LIN, "ban", Some(600)));
    napcat.send(group_ban(PLAIN, 0, "ban", Some(600)));
    napcat.send(group_ban(PLAIN, BOT, "ban", None));
    admin_calls(&napcat, PLAIN, 1);
    assert_eq!(napcat.group_message(PLAIN).await, [words("在")]);
    let events = until_count(&home, PLAIN, "venue.delivered", 1).await;
    assert!(of_kind(&events, MUTED).is_empty(), "{events:#?}");
    assert_eq!(decided(&home, PLAIN, 1).await["outcome"], "reply");
    // 禁的是她才认。
    napcat.send(group_ban(PLAIN, BOT, "ban", Some(600)));
    let events = until_count(&home, PLAIN, MUTED, 1).await;
    assert_eq!(of_kind(&events, MUTED).len(), 1);
    stopped(home).await;
}

#[tokio::test]
async fn what_waits_in_the_queue_counts_for_repeats() {
    let (release, released) = oneshot::channel();
    let lines = Lines::new([
        Line::calls("我在看这个问题").released_by(released),
        Line::calls("我在看这个问题！"),
        Line::says("看完了"),
    ]);
    let (home, mut napcat, _) = started_by(Arc::new(lines), RULES, MEMBERS).await;
    admin_calls(&napcat, PLAIN, 1);
    until_count(&home, PLAIN, "turn.started", 1).await;
    napcat.send(group_ban(PLAIN, BOT, "ban", Some(600)));
    until_count(&home, PLAIN, MUTED, 1).await;
    release.send(()).expect("她在等");
    // 等入队的事件本身：`turn.ended` 记下的时候，桥可能还没办完她最后一句。最后一句入队了，前面那句差个标点的也办完了。
    let events = until_count(&home, PLAIN, QUEUED, 2).await;
    let texts: Vec<Value> = of_kind(&events, QUEUED)
        .iter()
        .map(|one| one["body"]["text"].clone())
        .collect();
    assert_eq!(
        texts,
        ["我在看这个问题", "看完了"],
        "排着没发的也算这一回合说过的"
    );
    assert!(run_log(&home.root).contains("why=repeated"));
    napcat.send(group_ban(PLAIN, BOT, "lift_ban", None));
    assert_eq!(
        napcat.group_message(PLAIN).await.last(),
        Some(&words("我在看这个问题"))
    );
    assert_eq!(
        napcat.group_message(PLAIN).await.last(),
        Some(&words("看完了"))
    );
    until_count(&home, PLAIN, "venue.delivered", 2).await;
    assert!(napcat.pending().is_none());
    stopped(home).await;
}

/// 本机此刻，毫秒。
fn now_millis() -> i64 {
    let since = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("钟在 1970 年以后");
    i64::try_from(since.as_millis()).expect("放得下")
}
