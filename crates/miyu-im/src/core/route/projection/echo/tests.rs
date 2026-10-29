//! 复读和「她已经在回的」（施工 I-28）：别人刚说过同一句的认得出是复读，自己重复、隔得太久的不算；她回的那一条后来有别人跟着说
//! 的认得出；判过要回、还没回完的几条交给判官，写着名字和正文的开头，这一次在判的不算。

use miyu_client::events::Event;
use serde_json::json;

use super::super::Projection;
use super::super::tests::{LINE, at, decided, event};

/// 窗口：两分钟。
const WINDOW: i64 = 120_000;

/// 号是 `user`、名字叫 `name` 的人第 `seconds` 秒在群里说 `text`。
fn said(seq: u64, seconds: i64, user: i64, text: &str) -> Event {
    let by = json!({"kind": "external", "venue": "qq:30003:group:5", "id": format!("qq:{user}")});
    let body = json!({
        "blocks": [{"type": "text", "text": text}],
        "venue": {"msg": seq.to_string(), "ambient": true, "name": format!("阿{user}")},
    });
    event(seq, seconds, "message.user", None, by, body)
}

/// 收下 `events` 的投影。
fn projected(events: &[Event]) -> Projection {
    let mut projection = Projection::new(LINE, 0);
    for one in events {
        projection.take(one);
    }
    projection
}

/// 序号是 `seq` 的那一条在它自己的时刻（第 `seconds` 秒）是不是复读；此刻没有在判的。
fn repeated(projection: &Projection, seq: u64, seconds: i64) -> bool {
    projection.repeated(seq, at(seconds), WINDOW, &[])
}

#[test]
fn a_line_someone_else_just_said_is_a_repeat() {
    let projection = projected(&[
        said(1, 0, 101, "miyu似了"),
        decided(2, &[1], "reply"),
        said(3, 6, 102, "Miyu 似了！"),
        said(4, 9, 103, "miyu似了"),
        said(5, 10, 104, "早"),
    ]);
    assert!(!repeated(&projection, 1, 0), "第一条不是");
    assert!(repeated(&projection, 3, 6));
    assert!(repeated(&projection, 4, 9));
    assert!(!repeated(&projection, 5, 10), "说的不一样");
    assert!(!repeated(&projection, 9, 10), "不在投影里的不算");
}

#[test]
fn only_a_judged_line_makes_the_next_one_a_repeat() {
    // 前面那一条没判过的，后面的照常判；正在判（还没记下判断）的算判过；睡着时只记下的（进站链没放行）不算。
    let fresh = projected(&[said(1, 0, 101, "在吗"), said(2, 5, 102, "在吗")]);
    assert!(!repeated(&fresh, 2, 5), "前面那一条没判过");
    assert!(fresh.repeated(2, at(5), WINDOW, &[1]), "正在判的算");
    let body = json!({"msgs": [1], "standing": "member", "inbound": "record_only", "why": "asleep", "outcome": "record"});
    let asleep = event(
        2,
        1,
        "ext.im.chat.decided",
        None,
        json!({"kind": "module", "id": "im"}),
        body,
    );
    let slept = projected(&[said(1, 0, 101, "在吗"), asleep, said(3, 5, 102, "在吗")]);
    assert!(!repeated(&slept, 3, 5), "只记下的不算判过");
}

#[test]
fn saying_it_again_yourself_or_much_later_is_not_a_repeat() {
    let projection = projected(&[
        said(1, 0, 101, "miyu似了"),
        said(2, 5, 101, "miyu似了"),
        said(3, 200, 102, "miyu似了"),
    ]);
    assert!(!repeated(&projection, 2, 5), "自己补发的");
    assert!(!repeated(&projection, 3, 200), "隔了三分多钟");
}

#[test]
fn a_message_others_echo_afterwards_is_known() {
    let projection = projected(&[
        said(1, 0, 101, "miyu似了"),
        said(2, 3, 104, "早"),
        said(3, 6, 102, "miyu似了"),
    ]);
    assert!(projection.echoed(1));
    assert!(!projection.echoed(2));
    assert!(!projection.echoed(3), "后面没人再说");
}

#[test]
fn what_she_has_decided_to_answer_goes_to_the_judge() {
    let projection = projected(&[
        said(1, 0, 101, "miyu似了"),
        said(2, 1, 102, "今晚吃什么"),
        decided(3, &[1], "reply"),
        decided(4, &[2], "record"),
    ]);
    let answering = projection.answering(&[]);
    assert_eq!(answering.len(), 1, "只记下的不算：{answering:?}");
    assert_eq!(answering[0].name, "阿101");
    assert_eq!(answering[0].text, "miyu似了");
    assert!(projection.answering(&[1]).is_empty(), "这一次在判的不算");
}
