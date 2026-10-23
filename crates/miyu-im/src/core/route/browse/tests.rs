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
fn each_message_gets_its_latest_judgement() {
    let events = json!([
        {"kind": "message.user", "seq": 5},
        {"kind": "ext.im.chat.decided", "body": {"msgs": [5], "outcome": "record", "inbound": "pass", "judge": {"reason": "在跟别人说"}}},
        {"kind": "ext.im.chat.decided", "body": {"msgs": [6, 7], "outcome": "record", "why": "repeated", "inbound": "record_only"}},
        {"kind": "ext.im.chat.decided", "body": {"msgs": [5], "outcome": "reply", "inbound": "pass", "judge": {"reason": "问她的"}}},
        {"kind": "ext.im.chat.decided", "body": {"msgs": [8], "outcome": "record", "inbound": "rate_limited"}},
    ]);
    let judged = judged(&events);
    assert_eq!(
        judged["m5"],
        json!({"outcome": "reply", "why": null, "reason": "问她的"}),
        "同一条照后一次"
    );
    assert_eq!(
        judged["m6"],
        json!({"outcome": "record", "why": "repeated", "reason": null})
    );
    assert_eq!(judged["m7"], judged["m6"], "一笔管几条的每条都记");
    assert_eq!(
        judged["m8"]["why"], "rate_limited",
        "没写 why 的照进站链的结果"
    );
    assert_eq!(judged.len(), 4);
}
