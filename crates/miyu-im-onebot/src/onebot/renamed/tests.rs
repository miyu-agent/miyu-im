//! 群名片改了怎么认（施工 I-38）。

use serde_json::json;

use super::*;

#[test]
fn a_new_card_is_read_and_a_cleared_one_is_empty() {
    let frame = json!({"post_type": "notice", "notice_type": "group_card", "self_id": 9, "group_id": 31, "user_id": 9,
        "card_new": "米鱼", "card_old": "Miyu"});
    assert_eq!(
        renamed(&frame),
        Some(Event::Renamed {
            bot: 9,
            group: 31,
            user: 9,
            card: Some("米鱼".to_string())
        })
    );
    for cleared in [json!(""), json!("  "), json!(null)] {
        let mut frame = frame.clone();
        frame["card_new"] = cleared;
        assert!(
            matches!(renamed(&frame), Some(Event::Renamed { card: None, .. })),
            "{frame}"
        );
    }
}

#[test]
fn other_notices_and_unreadable_numbers_are_not_renames() {
    assert_eq!(
        renamed(&json!({"notice_type": "group_ban", "self_id": 9, "group_id": 31, "user_id": 7})),
        None
    );
    assert_eq!(
        renamed(&json!({"notice_type": "group_card", "self_id": 9, "user_id": 7, "card_new": "x"})),
        None,
        "没有群号"
    );
}
