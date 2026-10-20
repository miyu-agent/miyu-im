use serde_json::json;

use super::{groups, skipped};

#[test]
fn listed_groups_and_their_skipped_people_are_read_from_the_cards() {
    let cards = json!({"cards": {"groups": [
        {"id": 777, "set": {"rules": {"chatty.skip_users": ["qq:1", "qq:2"]}}},
        {"id": "888", "set": {}},
        {"set": {}}
    ]}});
    let found = groups(&cards);
    let ids: Vec<&str> = found.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["777", "888"]);
    assert_eq!(skipped(found[0].1), ["qq:1", "qq:2"]);
    assert!(skipped(found[1].1).is_empty());
}
