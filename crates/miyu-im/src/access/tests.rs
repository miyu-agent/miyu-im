use serde_json::json;

use super::{groups, skipped};
use crate::chat::Platform;

#[test]
fn listed_groups_and_their_skipped_people_are_read_from_the_cards() {
    let cards = json!({"cards": {"groups": [
        {"id": 777, "set": {"rules": {"chatty.skip_users": ["qq:1", "qq:2"]}}},
        {"id": "888", "set": {}},
        {"set": {}},
        {"platform": "tg", "id": -1001, "set": {}}
    ]}});
    let found = groups(&cards, Platform::new("qq"));
    let ids: Vec<&str> = found.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["777", "888"], "没写平台的是 QQ 的");
    // 施工 AUD-4（审计 #114）：照平台分。
    let telegram = groups(&cards, Platform::new("tg"));
    assert_eq!(
        telegram
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>(),
        ["-1001"]
    );
    assert_eq!(skipped(found[0].1), ["qq:1", "qq:2"]);
    assert!(skipped(found[1].1).is_empty());
}
