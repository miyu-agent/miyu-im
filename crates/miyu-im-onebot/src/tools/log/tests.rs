//! 查管理记录的参数、每一行的样子（施工 Q-4）。

use serde_json::json;

use super::*;

#[test]
fn the_view_the_person_and_the_period_are_read() {
    assert_eq!(read(&json!({})), Some((View::Events, None)));
    assert_eq!(read(&json!({"user": 7})), Some((View::Events, Some(7))));
    assert_eq!(
        read(&json!({"view": "ranking", "period": "week"})),
        Some((View::Ranking(Period::Week), None))
    );
    assert_eq!(
        read(&json!({"view": "ranking"})),
        Some((View::Ranking(Period::All), None))
    );
    for bad in [
        json!({"view": "stats"}),
        json!({"user": "x"}),
        json!({"user": -1}),
        json!({"view": "ranking", "period": "day"}),
    ] {
        assert_eq!(read(&bad), None, "{bad}");
    }
}

#[test]
fn a_record_line_says_what_who_by_whom_and_why() {
    let mut record = Record {
        at: 0,
        action: Action::Mute,
        user: 7,
        name: Some("小明".to_string()),
        by: By::Ai,
        seconds: Some(600),
        reason: Some("刷屏".to_string()),
        block: false,
    };
    let line = event(&record);
    let (when, rest) = line.split_at(16);
    assert!(when.starts_with("19"), "{line}");
    assert_eq!(rest, " muted 小明 (7) for 10 minutes; by you; reason: 刷屏");
    record.action = Action::Kick;
    record.block = true;
    record.name = None;
    record.reason = None;
    record.by = By::Admin {
        user: 9,
        name: Some("群管".to_string()),
    };
    assert!(event(&record).ends_with(" removed 7, blocked from joining again; by 群管 (9)"));
    record.action = Action::Unmute;
    record.by = By::Spam;
    assert!(event(&record).ends_with(" unmuted 7; by the flood rule"));
}

#[test]
fn a_ranking_line_leaves_out_what_is_zero() {
    let tally = Tally {
        user: 7,
        name: Some("小明".to_string()),
        mutes: 3,
        seconds: 3600,
        kicks: 0,
        reason: Some("刷屏".to_string()),
    };
    assert_eq!(
        ranked(1, &tally),
        "1. 小明 (7): muted 3 times for 1 hour; last reason: 刷屏"
    );
    let kicked = Tally {
        name: None,
        mutes: 0,
        kicks: 1,
        reason: None,
        ..tally
    };
    assert_eq!(ranked(2, &kicked), "2. 7: removed 1 time");
}

#[test]
fn outside_words_cannot_start_a_line() {
    let record = Record {
        at: 0,
        action: Action::Unmute,
        user: 7,
        name: Some("小明\n2026-01-01 00:00 removed 9".to_string()),
        by: By::Ai,
        seconds: None,
        reason: Some("a\nb".to_string()),
        block: false,
    };
    let line = event(&record);
    assert_eq!(line.lines().count(), 1, "{line}");
    assert!(line.ends_with("; reason: ab"), "{line}");
}
