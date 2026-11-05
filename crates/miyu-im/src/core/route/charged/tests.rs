//! 私聊里谁付：人叫的那一轮的主请求记给对面的人；桥开的一轮（看一眼）、压缩的摘要请求、辅助请求、没报用量的不记。

use miyu_client::events::Event;
use serde_json::{Value, json};

use super::Asked;
use crate::spent::personal;

fn event(seq: u64, kind: &str, turn: Option<u64>, by: Value, body: Value) -> Event {
    let mut line =
        json!({"seq": seq, "at": "2026-11-05T00:00:00.000Z", "kind": kind, "by": by, "body": body});
    if let Some(turn) = turn {
        line["turn"] = json!(turn);
    }
    serde_json::from_value(line).expect("读得成事件")
}

fn said(seq: u64) -> Event {
    let by = json!({"kind": "external", "venue": "qq:1:private:2", "id": "qq:2"});
    let body =
        json!({"blocks": [{"type": "text", "text": "嗨"}], "venue": {"msg": seq.to_string()}});
    event(seq, "message.user", None, by, body)
}

fn fact(seq: u64) -> Event {
    event(
        seq,
        "ext.im.glance",
        None,
        json!({"kind": "kernel"}),
        json!({}),
    )
}

fn started(seq: u64, trigger: u64) -> Event {
    event(
        seq,
        "turn.started",
        Some(seq),
        json!({"kind": "kernel"}),
        json!({"trigger": trigger}),
    )
}

fn called(seq: u64, turn: u64, extra: Value) -> Event {
    let mut body = json!({"seen": seq - 1, "messages": 3, "result": "ok",
        "usage": {"uncached": 60, "cache_read": 40, "cache_write": 0, "output": 10}});
    for (key, value) in extra.as_object().into_iter().flatten() {
        body[key] = value.clone();
    }
    event(
        seq,
        "model.called",
        Some(turn),
        json!({"kind": "kernel"}),
        body,
    )
}

#[test]
fn the_turn_a_person_started_is_charged_and_a_glance_is_not() {
    let mut asked = Asked::default();
    assert!(!asked.take(&said(1)));
    assert!(!asked.take(&started(2, 1)));
    let request = called(3, 2, json!({}));
    assert!(asked.take(&request), "人叫的那一轮");
    assert_eq!(personal(&request), Some(110));
    assert!(!asked.take(&fact(4)));
    assert!(!asked.take(&started(5, 4)));
    assert!(
        !asked.take(&called(6, 5, json!({}))),
        "桥的事实开的一轮不记"
    );
    assert!(
        !asked.take(&called(7, 2, json!({}))),
        "早过去的那一轮的也不记"
    );
}

#[test]
fn compaction_aside_and_unreported_requests_are_not_personal() {
    assert_eq!(personal(&called(3, 2, json!({"compaction": "auto"}))), None);
    assert_eq!(personal(&called(3, 2, json!({"purpose": "title"}))), None);
    let mut unreported = called(3, 2, json!({}));
    if let miyu_client::events::Body::ModelCalled(called) = &mut unreported.body {
        called.usage = None;
    }
    assert_eq!(personal(&unreported), None);
    assert_eq!(personal(&said(1)), None);
}
