//! 禁言、踢人归群管理（施工 Q-1、Q-2，设计 02 第四节）：对谁做照 `target`、调动作照 `acting`，这里是群管理加的那几样：插件在这个
//! 群开着、秒数照设置的最短最长（[`checked`]），要做之前问她在这个群的身份（[`admin_here`]），做成了记进管理记录（[`recorded`]，
//! 记录开关关了的不记；AI 自己做的不另给会话记事实：工具结果已经在她的上下文里）。

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use miyu_im::chat::{CallError, Link};
use miyu_im::toolset::{Call, NAME, clean, duration};

use super::acting::Act;
use crate::admin::{self, records};

/// 群管理在这个群没生效（插件关着、她不是管理员）答的那一句（施工 Q-1）。
pub(super) const ADMIN_OFF: &str = "admin-off";

/// 理由最多记几个字符（她给的一句话）。
const REASON: usize = 200;

/// 禁言、踢人当场挡下的：插件在这个群没开着的答 `admin-off`，禁言秒数不在设置的最短、最长中间的答 `bad-seconds`（解禁不管）。别的
/// 动作、过了的是空的。
pub(super) fn checked(call: &Call, act: &Act) -> Option<Value> {
    if !matches!(act, Act::Mute { .. } | Act::Kick { .. }) {
        return None;
    }
    if !call.plugins.contains(&admin::ID) {
        return Some(call.answers.say(ADMIN_OFF, &[], true));
    }
    let (min, max) = admin::mute_range(&call.config);
    match act {
        Act::Mute { seconds, .. } if *seconds != 0 && !(min..=max).contains(seconds) => {
            Some(bad_seconds(call))
        }
        _ => None,
    }
}

/// 秒数写错、出了范围答的那一句：带设置的最短、最长（施工 Q-1）。
pub(super) fn bad_seconds(call: &Call) -> Value {
    let (min, max) = admin::mute_range(&call.config);
    let (min, max) = (duration(min), duration(max));
    call.answers
        .say("bad-seconds", &[("min", &min), ("max", &max)], true)
}

/// 她（机器人号 `bot`）在群 `group` 里管得了人没有：问 NapCat 她的身份，群主、管理员才算（施工 Q-1）。问不到的交回为什么。
pub(super) async fn admin_here(link: &dyn Link, group: i64, bot: i64) -> Result<bool, CallError> {
    let me = link.member(group, bot).await?;
    Ok(admin::ranked(me.rank))
}

/// 做成了的禁言、解禁、踢人记进这个群的管理记录（施工 Q-2）：对谁、那时叫什么、她给的理由；记录开关关了的不记。
pub(super) async fn recorded(call: &Call, act: &Act) {
    if !admin::recording(&call.config) {
        return;
    }
    let (group, user, action, seconds, block) = match *act {
        Act::Mute {
            group,
            user,
            seconds: 0,
        } => (group, user, records::Action::Unmute, None, false),
        Act::Mute {
            group,
            user,
            seconds,
        } => (group, user, records::Action::Mute, Some(seconds), false),
        Act::Kick { group, user, block } => (group, user, records::Action::Kick, None, block),
        _ => return,
    };
    let record = records::Record {
        at: now(),
        action,
        user,
        name: call.names.get(&user).and_then(|name| clean(name, NAME)),
        by: records::By::Ai,
        seconds,
        reason: call.args["reason"]
            .as_str()
            .and_then(|reason| clean(reason.trim(), REASON)),
        block,
    };
    records::append(call.store.clone(), group, record, admin::keep(&call.config)).await;
}

/// 本机此刻的 Unix 秒；钟在 1970 年以前的是 0。
pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}
