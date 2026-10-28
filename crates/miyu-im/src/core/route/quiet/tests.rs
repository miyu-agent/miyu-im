//! 这一轮不说话了（施工 O-26，`onebot.md` 第一条「提供者和不说话」第 3 条）：推来的事件照原样的 JSON 交进来。

use serde_json::{Value, json};

use super::Quiet;

/// 她在回合 `turn` 的一条回复，块是 `blocks`。
fn reply(turn: u64, blocks: Value) -> Value {
    json!({"seq": 10, "kind": "message.assistant", "turn": turn, "body": {"blocks": blocks}})
}

/// 一块字。
fn text(text: &str) -> Value {
    json!({"type": "text", "text": text})
}

/// 一块调用：调 `name`。
fn call(name: &str) -> Value {
    json!({"type": "tool_call", "call_id": "c1", "name": name, "args": "{\"reason\":\"x\"}"})
}

/// 回合 `turn` 结束了。
fn ended(turn: u64) -> Value {
    json!({"seq": 20, "kind": "turn.ended", "turn": turn, "body": {"reason": "done"}})
}

#[test]
fn a_reply_that_calls_skip_reply_hushes_the_rest_of_its_turn() {
    let mut quiet = Quiet::new(Some("skip_reply"));
    assert!(
        !quiet.heard("s1", &reply(5, json!([text("先说一句")]))),
        "调用以前的照发"
    );
    assert!(
        quiet.heard("s1", &reply(5, json!([text("不该发"), call("skip_reply")]))),
        "同一条里的字不发"
    );
    assert!(
        quiet.heard("s1", &reply(5, json!([text("也不发")]))),
        "这一轮以后的不发"
    );
    assert!(
        !quiet.heard("s1", &reply(9, json!([text("别的回合")]))),
        "别的回合照发"
    );
    assert!(
        !quiet.heard("s2", &reply(5, json!([text("别的会话")]))),
        "别的会话照发：回合编号各数各的"
    );
}

#[test]
fn other_calls_and_plain_words_do_not() {
    let mut quiet = Quiet::new(Some("skip_reply"));
    assert!(!quiet.heard("s1", &reply(5, json!([text("x"), call("no_such_tool")]))));
    assert!(!quiet.heard("s1", &reply(5, json!([text("skip_reply")]))));
    assert!(!quiet.heard(
        "s1",
        &reply(5, json!([{"type": "reasoning", "text": "skip_reply"}]))
    ));
    assert!(!quiet.heard("s1", &reply(5, json!([text("y")]))));
}

#[test]
fn the_end_of_the_turn_clears_it() {
    let mut quiet = Quiet::new(Some("skip_reply"));
    assert!(quiet.heard("s1", &reply(5, json!([call("skip_reply")]))));
    assert!(quiet.heard("s1", &reply(7, json!([call("skip_reply")]))));
    assert!(!quiet.heard("s1", &ended(5)), "不是她的回复");
    assert!(
        !quiet.heard("s1", &reply(5, json!([text("x")]))),
        "那一轮清掉了"
    );
    assert!(
        quiet.heard("s1", &reply(7, json!([text("y")]))),
        "别的回合的不清"
    );
    assert!(!quiet.heard("s2", &ended(7)), "别的会话的结束");
    assert!(
        quiet.heard("s1", &reply(7, json!([text("z")]))),
        "别的会话的结束不清这一个"
    );
}

#[test]
fn events_without_a_turn_are_not_looked_at() {
    let mut quiet = Quiet::new(Some("skip_reply"));
    let mut lone = reply(5, json!([call("skip_reply")]));
    lone.as_object_mut().expect("是对象").remove("turn");
    assert!(!quiet.heard("s1", &lone));
    assert!(!quiet.heard("s1", &reply(5, json!([text("x")]))));
}

#[test]
fn held_words_are_kept_per_line_and_turn_and_dropped_when_she_stays_quiet() {
    // 仅发送最终消息（施工 O-57）：扣着的照（线，回合）放、取；同一回合再扣的换掉；这一轮不说话了的丢掉。
    use super::super::voicing::Voice;
    use super::{Held, has_calls};
    let held = |text: &str| Held {
        session: "s".into(),
        voice: Voice::Private { turn: 1 },
        text: text.into(),
    };
    let mut quiet = Quiet::new(Some("skip_reply"));
    quiet.hold("s", 1, held("一"));
    quiet.hold("s", 1, held("二"));
    quiet.hold("lane", 1, held("支线的"));
    assert_eq!(
        quiet.release("s", 1).map(|held| held.text),
        Some("二".into())
    );
    assert!(quiet.release("s", 1).is_none(), "取了就不再扣着");
    assert_eq!(
        quiet.release("lane", 1).map(|held| held.text),
        Some("支线的".into())
    );
    quiet.hold("s", 2, held("三"));
    quiet.heard("s", &reply(2, json!([call("skip_reply")])));
    assert!(quiet.release("s", 2).is_none(), "不说话了：扣着的丢掉");
    assert!(has_calls(&reply(3, json!([text("a"), call("search")]))));
    assert!(!has_calls(&reply(3, json!([text("a")]))));
    assert!(!has_calls(&ended(3)));
}

#[test]
fn every_drivers_silence_tool_hushes_the_turn() {
    // 挂几个驱动的（施工 I-70 中上）：每个驱动的「这一轮不说话」都认，别的工具不算。
    let mut quiet = Quiet::new(["qq_skip_reply", "tg_skip_reply"]);
    assert!(quiet.heard("s1", &reply(1, json!([call("tg_skip_reply")]))));
    assert!(quiet.heard("s2", &reply(1, json!([call("qq_skip_reply")]))));
    assert!(!quiet.heard("s3", &reply(1, json!([call("tg_send"), text("在")]))));
}
