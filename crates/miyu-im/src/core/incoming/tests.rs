//! 读进来的一条：事件推送留原文、认得出会话，取的时候解得回原样；有编号的、别的方法、回应照旧当场解；不是 JSON 的报错；取出来的
//! 照先后。

use serde_json::json;

use super::{Incoming, Taken};

#[test]
fn event_pushes_keep_the_line_and_parse_back_unchanged() {
    let pushed = json!({"jsonrpc": "2.0", "method": "event", "params": {"session": "s1", "event": {"seq": 7, "kind": "message.user"}}});
    let line = format!("{pushed}\n");
    let read = Incoming::read(&line).expect("是 JSON");
    assert!(matches!(&read, Incoming::Event { raw, .. } if !raw.ends_with('\n')));
    assert!(read.event_of(|one| one == "s1"));
    assert!(!read.event_of(|one| one == "s2"));
    assert_eq!(read.value(), pushed);
}

#[test]
fn replies_requests_and_other_pushes_are_parsed_at_once() {
    for message in [
        json!({"jsonrpc": "2.0", "id": "im-1", "result": {}}),
        json!({"jsonrpc": "2.0", "id": "c1", "method": "event", "params": {"session": "s1"}}),
        json!({"jsonrpc": "2.0", "method": "extension.config", "params": {"keys": {}}}),
    ] {
        let read = Incoming::read(&message.to_string()).expect("是 JSON");
        assert!(
            matches!(&read, Incoming::Parsed(value) if *value == message),
            "{message}"
        );
        assert!(!read.event_of(|_| true));
    }
    assert!(Incoming::read("not json").is_err());
}

#[test]
fn an_event_with_an_escaped_session_is_still_recognised() {
    let line = r#"{"method":"event","params":{"session":"a\"b","event":{}}}"#;
    let read = Incoming::read(line).expect("是 JSON");
    assert!(read.event_of(|one| one == "a\"b"));
}

#[test]
fn taken_parses_in_order() {
    let lines = [1, 2, 3].map(|seq| {
        json!({"method": "event", "params": {"session": "s", "event": {"seq": seq}}}).to_string()
    });
    let taken = Taken(
        lines
            .iter()
            .map(|line| Incoming::read(line).expect("JSON"))
            .collect(),
    );
    let seqs: Vec<u64> = taken
        .map(|value| value["params"]["event"]["seq"].as_u64().expect("有序号"))
        .collect();
    assert_eq!(seqs, [1, 2, 3]);
}
