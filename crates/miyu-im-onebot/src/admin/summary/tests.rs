//! 成员汇总和排行榜（施工 Q-4）。

use super::*;
use crate::admin::records::By;

fn record(
    at: u64,
    action: Action,
    user: i64,
    seconds: Option<u64>,
    reason: Option<&str>,
) -> Record {
    Record {
        at,
        action,
        user,
        name: Some(format!("u{user}")),
        by: By::Ai,
        seconds,
        reason: reason.map(str::to_string),
        block: false,
    }
}

const NOW: u64 = 100 * 86_400;

#[test]
fn people_are_ranked_by_mutes_then_length_then_kicks() {
    let records = [
        record(NOW - 40 * 86_400, Action::Mute, 7, Some(60), Some("old")),
        record(NOW - 3 * 86_400, Action::Mute, 7, Some(600), Some("刷屏")),
        record(NOW - 2 * 86_400, Action::Unmute, 7, None, None),
        record(NOW - 86_400, Action::Mute, 8, Some(60), None),
        record(NOW - 86_400, Action::Mute, 8, Some(60), Some("广告")),
        record(NOW - 10, Action::Kick, 9, None, Some("骂人")),
        record(NOW - 5, Action::Unmute, 10, None, None),
        record(NOW - 10 * 86_400, Action::Mute, 11, Some(60), None),
    ];
    let all = ranking(&records, Period::All, NOW);
    let order: Vec<(i64, u32, u64, u32)> = all
        .iter()
        .map(|tally| (tally.user, tally.mutes, tally.seconds, tally.kicks))
        .collect();
    assert_eq!(
        order,
        [(7, 2, 660, 0), (8, 2, 120, 0), (11, 1, 60, 0), (9, 0, 0, 1)],
        "只解禁的不算"
    );
    assert_eq!(all[0].reason.as_deref(), Some("刷屏"), "最近的理由");
    assert_eq!(all[1].reason.as_deref(), Some("广告"));
    assert_eq!(all[0].name.as_deref(), Some("u7"));
    let week: Vec<(i64, u32)> = ranking(&records, Period::Week, NOW)
        .iter()
        .map(|tally| (tally.user, tally.mutes))
        .collect();
    assert_eq!(week, [(8, 2), (7, 1), (9, 0)], "一周以前的不算");
    let month: Vec<i64> = ranking(&records, Period::Month, NOW)
        .iter()
        .map(|tally| tally.user)
        .collect();
    assert_eq!(
        month,
        [8, 7, 11, 9],
        "十天以前的算进一个月，四十天以前的不算"
    );
}

#[test]
fn periods_are_read_and_named() {
    for period in [Period::Week, Period::Month, Period::All] {
        assert_eq!(Period::read(period.name()), Some(period));
    }
    assert_eq!(Period::read("day"), None);
}
