//! 群里的事（设计 04，施工 C-1 上补）：NapCat 推来的通知认成 [`Event::Happened`]，只记进聊天记录。
//!
//! - `group_ban`：禁的不是她（她自己的照 `ban` 认成 `Muted`、`Unmuted`），也不是全员禁言（`user_id` 是 0）。`ban` 带的 `duration`
//!   大于 0 是禁言，0 是解禁；`lift_ban` 是解禁。
//! - `group_decrease`：`leave` 自己退群，`kick` 被踢，`kick_me` 是她被踢（对谁写她）。
//! - `group_increase`：`approve`、`invite` 都是进群。
//!
//! 谁做的照 `operator_id`，0 和没带的当不知道；退群的不写谁做的。

use serde_json::Value;

use miyu_im::chat::{Event, GroupChange, Happened};

use super::{id, number, positive};

/// 认一帧通知；不是这几种的是空的。
pub(super) fn happened(frame: &Value) -> Option<Event> {
    let (bot, group, user) = (
        id(&frame["self_id"])?,
        id(&frame["group_id"])?,
        id(&frame["user_id"])?,
    );
    let operator = positive(&frame["operator_id"]);
    let sub = frame["sub_type"].as_str()?;
    let (change, user, seconds) = match (frame["notice_type"].as_str()?, sub) {
        ("group_ban", _) if user == bot || user.number() == Some(0) => return None,
        ("group_ban", "ban") => match u64::try_from(number(&frame["duration"])?).ok()? {
            0 => (GroupChange::Unmuted, user, None),
            seconds => (GroupChange::Muted, user, Some(seconds)),
        },
        ("group_ban", "lift_ban") => (GroupChange::Unmuted, user, None),
        ("group_decrease", "leave") => (GroupChange::Left, user, None),
        ("group_decrease", "kick") => (GroupChange::Kicked, user, None),
        ("group_decrease", "kick_me") => (GroupChange::Kicked, bot.clone(), None),
        ("group_increase", "approve" | "invite") => (GroupChange::Joined, user, None),
        _ => return None,
    };
    let operator = if change == GroupChange::Left {
        None
    } else {
        operator
    };
    Some(Event::Happened(Happened {
        bot,
        group,
        change,
        user,
        operator,
        seconds,
    }))
}

#[cfg(test)]
mod tests;
