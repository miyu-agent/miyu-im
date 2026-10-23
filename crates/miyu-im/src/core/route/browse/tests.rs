//! 平台会话页的纯逻辑（施工 I-34 上）：会话照机器人号分组、一页原始事件里的判断。

use super::*;

fn known(pairs: &[(&str, i64, To)]) -> HashMap<String, Known> {
    pairs
        .iter()
        .map(|(session, bot, to)| (session.to_string(), Known { bot: *bot, to: *to }))
        .collect()
}

#[test]
fn sessions_are_grouped_by_the_bot_that_took_them_in() {
    let sessions = vec![
        json!({"session": "s1", "venue": "qq:group:31"}),
        json!({"session": "s2", "venue": "qq:private:7"}),
        json!({"session": "s3", "venue": "qq:group:32"}),
        json!({"session": "s4", "venue": "qq:private:10001"}),
    ];
    let known = known(&[
        ("s1", 100, To::Group(31)),
        ("s2", 200, To::Private(7)),
        ("s3", 100, To::Group(32)),
    ]);
    let groups = HashMap::from([(31, "测试群".to_string())]);
    let friends = HashMap::from([(7, "小明".to_string())]);
    let answer = grouped(&sessions, &known, (&groups, &friends), "qq");
    assert_eq!(
        answer,
        json!({"connections": [
            {"platform": "qq", "bot": 100, "sessions": [
                {"session": "s1", "venue": "qq:group:31", "kind": "group", "id": 31, "name": "测试群"},
                {"session": "s3", "venue": "qq:group:32", "kind": "group", "id": 32, "name": null},
            ]},
            {"platform": "qq", "bot": 200, "sessions": [
                {"session": "s2", "venue": "qq:private:7", "kind": "private", "id": 7, "name": "小明"},
            ]},
            {"platform": "qq", "bot": null, "sessions": [
                {"session": "s4", "venue": "qq:private:10001", "kind": "unknown", "id": null, "name": null},
            ]},
        ]})
    );
}

#[test]
fn each_message_gets_its_latest_judgement_and_its_speaker() {
    let events = json!([
        {"kind": "message.user", "seq": 5, "body": {"venue": {"msg": "1", "name": " 小林 "}}},
        {"kind": "message.user", "seq": 6, "body": {"venue": {"msg": "2", "name": "  "}}},
        {"kind": "ext.im.chat.decided", "body": {"msgs": [5], "outcome": "record", "inbound": "pass", "route": "judge"}},
        {"kind": "ext.im.chat.decided", "body": {"msgs": [6, 7], "outcome": "record", "why": "repeated", "inbound": "record_only"}},
        {"kind": "ext.im.chat.decided", "body": {"msgs": [5], "outcome": "reply", "inbound": "pass", "route": "commit", "conditions": [{"kind": "direct"}]}},
    ]);
    let (judged, names) = marks(&events);
    assert_eq!(
        judged["m5"],
        json!({"outcome": "reply", "why": {"reason": "called"}}),
        "同一条照后一次"
    );
    assert_eq!(
        judged["m6"],
        json!({"outcome": "record", "why": {"reason": "repeated"}})
    );
    assert_eq!(judged["m7"], judged["m6"], "一笔管几条的每条都记");
    assert_eq!(judged.len(), 3);
    assert_eq!(names["m5"], "小林", "去掉两头的空白");
    assert!(names.get("m6").is_none(), "空白的名字不算");
}
