//! 认别的管理员做的、时长的说法（施工 Q-3）。

use serde_json::json;

use super::*;

/// 一帧通知：`notice_type`、`sub_type`，她是 1，别的照给的。
fn frame(kind: &str, sub: &str, user: i64, operator: i64, duration: Option<u64>) -> Value {
    let mut frame = json!({"post_type": "notice", "notice_type": kind, "sub_type": sub, "self_id": 1, "group_id": 31, "user_id": user, "operator_id": operator});
    if let Some(duration) = duration {
        frame["duration"] = json!(duration);
    }
    frame
}

#[test]
fn other_admins_mutes_unmutes_and_kicks_are_read() {
    let base = Moderated {
        bot: 1,
        group: 31,
        user: 7,
        operator: 9,
        seconds: Some(600),
        action: Action::Mute,
    };
    assert_eq!(
        read(&frame("group_ban", "ban", 7, 9, Some(600))),
        Some(base)
    );
    let unmuted = Moderated {
        seconds: None,
        action: Action::Unmute,
        ..base
    };
    assert_eq!(
        read(&frame("group_ban", "lift_ban", 7, 9, Some(0))),
        Some(unmuted)
    );
    assert_eq!(
        read(&frame("group_ban", "ban", 7, 9, Some(0))),
        Some(unmuted)
    );
    let kicked = Moderated {
        seconds: None,
        action: Action::Kick,
        ..base
    };
    assert_eq!(
        read(&frame("group_decrease", "kick", 7, 9, None)),
        Some(kicked)
    );
}

#[test]
fn hers_on_her_and_others_are_not() {
    assert_eq!(
        read(&frame("group_ban", "ban", 7, 1, Some(60))),
        None,
        "她做的"
    );
    assert_eq!(
        read(&frame("group_ban", "ban", 1, 9, Some(60))),
        None,
        "对她做的"
    );
    assert_eq!(
        read(&frame("group_ban", "ban", 0, 9, Some(60))),
        None,
        "全员禁言"
    );
    assert_eq!(
        read(&frame("group_ban", "ban", 7, 0, Some(60))),
        None,
        "不知道谁"
    );
    assert_eq!(
        read(&frame("group_ban", "ban", 7, 9, None)),
        None,
        "没带时长"
    );
    assert_eq!(
        read(&frame("group_decrease", "leave", 7, 9, None)),
        None,
        "自己走的"
    );
    assert_eq!(read(&frame("group_decrease", "kick_me", 1, 9, None)), None);
    assert_eq!(read(&frame("group_increase", "approve", 7, 9, None)), None);
}

#[test]
fn durations_are_said_in_days_hours_and_minutes() {
    assert_eq!(spoken(30), "1 minute");
    assert_eq!(spoken(60), "1 minute");
    assert_eq!(spoken(61), "2 minutes");
    assert_eq!(spoken(600), "10 minutes");
    assert_eq!(spoken(3600), "1 hour");
    assert_eq!(spoken(5400), "1 hour 30 minutes");
    assert_eq!(spoken(86_400 * 2 + 60), "2 days 1 minute");
    assert_eq!(spoken(2_592_000), "30 days");
}
