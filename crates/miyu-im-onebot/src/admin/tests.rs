//! 群管理的设置（施工 Q-1）。

use serde_json::{Map, Value, json};

use super::*;

fn config(pairs: Value) -> Map<String, Value> {
    pairs.as_object().cloned().expect("是对象")
}

#[test]
fn the_mute_tool_is_offered_while_both_switches_are_on() {
    assert!(offers_mute(&Map::new()), "没写的当开着");
    assert!(offers_mute(&config(
        json!({"admin_on": true, "admin_mute": true})
    )));
    assert!(!offers_mute(&config(json!({"admin_on": false}))));
    assert!(!offers_mute(&config(json!({"admin_mute": false}))));
}

#[test]
fn the_mute_range_reads_minutes_and_falls_back_to_the_shipped_one() {
    assert_eq!(mute_range(&Map::new()), (60, 2_592_000));
    assert_eq!(
        mute_range(&config(json!({"admin_mute_min": 2, "admin_mute_max": 120}))),
        (120, 7200)
    );
    assert_eq!(
        mute_range(&config(json!({"admin_mute_min": 5, "admin_mute_max": 1}))),
        (60, 2_592_000),
        "最短比最长还长的照出厂的"
    );
    assert_eq!(
        mute_range(&config(
            json!({"admin_mute_min": 0, "admin_mute_max": 99_999})
        )),
        (60, 2_592_000),
        "0 读不出、超过 30 天的照 30 天"
    );
    assert_eq!(
        mute_range(&config(
            json!({"admin_mute_min": "5", "admin_mute_max": -3})
        )),
        (60, 2_592_000),
        "不是正整数的读不出"
    );
}

#[test]
fn only_owners_and_admins_can_moderate() {
    assert!(ranked(Some(Rank::Owner)));
    assert!(ranked(Some(Rank::Admin)));
    assert!(!ranked(Some(Rank::Member)));
    assert!(!ranked(None));
}

#[test]
fn kick_records_and_keep_follow_their_settings() {
    assert!(offers_kick(&Map::new()));
    assert!(!offers_kick(&config(json!({"admin_on": false}))));
    assert!(!offers_kick(&config(json!({"admin_kick": false}))));
    assert!(
        offers_kick(&config(json!({"admin_mute": false}))),
        "各管各的"
    );
    assert!(recording(&Map::new()));
    assert!(!recording(&config(json!({"admin_records": false}))));
    assert_eq!(keep(&Map::new()), 500);
    assert_eq!(keep(&config(json!({"admin_keep": 20}))), 20);
    assert_eq!(keep(&config(json!({"admin_keep": 0}))), 500);
}
