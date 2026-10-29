//! 平台会话页的纯逻辑（施工 I-34 上）：会话照机器人号分组。

use super::*;

fn known(pairs: &[(&str, i64, To)]) -> HashMap<String, Known> {
    pairs
        .iter()
        .map(|(session, bot, to)| {
            let known = Known {
                bot: Id::from(*bot),
                to: to.clone(),
            };
            (session.to_string(), known)
        })
        .collect()
}

#[test]
fn sessions_are_grouped_by_the_bot_that_took_them_in() {
    let sessions = vec![
        json!({"session": "s1", "venue": "qq:30003:group:31"}),
        json!({"session": "s2", "venue": "qq:30003:private:7"}),
        json!({"session": "s3", "venue": "qq:30003:group:32"}),
        json!({"session": "s4", "venue": "qq:30003:private:10001"}),
    ];
    let known = known(&[
        ("s1", 100, To::Group(Id::from(31))),
        ("s2", 200, To::Private(Id::from(7))),
        ("s3", 100, To::Group(Id::from(32))),
    ]);
    let groups = HashMap::from([(Id::from(31), "测试群".to_string())]);
    let friends = HashMap::from([(Id::from(7), "小明".to_string())]);
    let answer = grouped(&sessions, &known, (&groups, &friends), "qq");
    assert_eq!(
        answer,
        json!({"connections": [
            {"platform": "qq", "bot": 100, "sessions": [
                {"session": "s1", "venue": "qq:30003:group:31", "kind": "group", "id": 31, "name": "测试群"},
                {"session": "s3", "venue": "qq:30003:group:32", "kind": "group", "id": 32, "name": null},
            ]},
            {"platform": "qq", "bot": 200, "sessions": [
                {"session": "s2", "venue": "qq:30003:private:7", "kind": "private", "id": 7, "name": "小明"},
            ]},
            {"platform": "qq", "bot": null, "sessions": [
                {"session": "s4", "venue": "qq:30003:private:10001", "kind": "unknown", "id": null, "name": null},
            ]},
        ]})
    );
}
