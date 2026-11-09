use serde_json::{Map, Value, json};

use super::merge;

fn status(connections: Value) -> Map<String, Value> {
    json!({"connections": connections})
        .as_object()
        .expect("对象")
        .clone()
}

#[test]
fn reported_fields_join_the_bots_card_and_unknown_ones_get_their_own() {
    let mut cards =
        vec![json!({"id": "tg:7", "platform": "tg", "bot": "7", "connected": true, "caps": []})];
    merge(
        &mut cards,
        &status(json!([
            {"index": 0, "token": "set", "connected": true, "bot": "7", "username": "miyu_bot", "name": "Miyu", "privacy": true},
            {"index": 1, "token": "set", "connected": false, "trouble": "token"},
            {"index": 2, "token": "set", "connected": false, "bot": "9", "name": "小助手", "username": "helper", "trouble": "elsewhere"},
        ])),
        ("tg", &["menu"]),
    );
    assert_eq!(cards[0]["username"], "miyu_bot");
    assert_eq!(cards[0]["trouble"], "privacy", "连着、隐私模式开着");
    assert_eq!(cards[0]["index"], 0);
    assert_eq!(
        cards[1],
        json!({"id": "tg:#1", "platform": "tg", "bot": "", "connected": false, "caps": ["menu"], "index": 1, "trouble": "token"})
    );
    assert_eq!(cards[2]["id"], "tg:9");
    assert_eq!(
        (cards[2]["name"].as_str(), cards[2]["trouble"].as_str()),
        (Some("小助手"), Some("elsewhere"))
    );
}

#[test]
fn port_tables_are_left_alone() {
    let mut cards = vec![json!({"id": "qq:1", "bot": "1"})];
    let before = cards.clone();
    merge(
        &mut cards,
        &status(json!([{"host": "0.0.0.0", "listen": 8301, "token": "set"}])),
        ("qq", &[]),
    );
    assert_eq!(cards, before);
}
