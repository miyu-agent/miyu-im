//! 群里的事怎么认（施工 C-1 上补）。

use serde_json::{Value, json};

use super::*;

fn frame(notice: &str, sub: &str, user: i64, operator: i64, duration: Option<i64>) -> Value {
    let mut frame = json!({"post_type": "notice", "notice_type": notice, "sub_type": sub, "self_id": 9, "group_id": 31, "user_id": user, "operator_id": operator});
    if let Some(duration) = duration {
        frame["duration"] = json!(duration);
    }
    frame
}

fn read(frame: &Value) -> Option<(GroupChange, i64, Option<i64>, Option<u64>)> {
    match happened(frame)? {
        Event::Happened(happened) => Some((
            happened.change,
            happened.user.number()?,
            happened.operator.and_then(|operator| operator.number()),
            happened.seconds,
        )),
        _ => None,
    }
}

#[test]
fn others_being_muted_kicked_leaving_and_joining_are_read() {
    assert_eq!(
        read(&frame("group_ban", "ban", 7, 8, Some(600))),
        Some((GroupChange::Muted, 7, Some(8), Some(600)))
    );
    assert_eq!(
        read(&frame("group_ban", "ban", 7, 8, Some(0))),
        Some((GroupChange::Unmuted, 7, Some(8), None)),
        "禁 0 秒是解禁"
    );
    assert_eq!(
        read(&frame("group_ban", "lift_ban", 7, 0, Some(0))),
        Some((GroupChange::Unmuted, 7, None, None)),
        "0 是不知道谁"
    );
    assert_eq!(
        read(&frame("group_decrease", "kick", 7, 8, None)),
        Some((GroupChange::Kicked, 7, Some(8), None))
    );
    assert_eq!(
        read(&frame("group_decrease", "kick_me", 9, 8, None)),
        Some((GroupChange::Kicked, 9, Some(8), None)),
        "她被踢"
    );
    assert_eq!(
        read(&frame("group_decrease", "leave", 7, 7, None)),
        Some((GroupChange::Left, 7, None, None)),
        "退群的不写谁做的"
    );
    assert_eq!(
        read(&frame("group_increase", "invite", 7, 8, None)),
        Some((GroupChange::Joined, 7, Some(8), None))
    );
}

#[test]
fn her_own_ban_whole_group_bans_and_others_are_not_read_here() {
    assert_eq!(
        read(&frame("group_ban", "ban", 9, 8, Some(60))),
        None,
        "她自己的照 ban 认"
    );
    assert_eq!(
        read(&frame("group_ban", "ban", 0, 8, Some(60))),
        None,
        "全员禁言"
    );
    assert_eq!(read(&frame("group_recall", "", 7, 7, None)), None);
    assert_eq!(read(&frame("group_increase", "weird", 7, 8, None)), None);
}
