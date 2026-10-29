//! 查管理记录（施工 Q-4，设计 02 第一节第 1、4 条）：`moderation_log {view?, user?, period?, action?, limit?}`，只给群。`events` 是这个
//! 群的记录，新的在前，最多 `limit` 条（施工 I-17：出厂 [`SHOWN`]，最多 [`MOST`]），写了 `user` 的只看这个人，写了 `action`（`mute`、
//! `unmute`、`kick`）的只看这一种，禁言带状态（禁言中、到期、解禁、被后一次覆盖，`records::mute_state`）；`ranking` 是成员汇总排的榜
//! （`period`：`week`、`month`、`all`，出厂 `all`），写了 `user` 的只看这个人那一行（成员汇总关了的答 `ranking-off`）。群管理在这个群
//! 生效才查（插件开着、她是管理员或群主）。参数写错的说哪个（施工 I-17）。跨群查、不是管理员也能查不做（设计 02 定的）。
//!
//! 每一行的样子在这里拼（同群成员列表）：时刻照本机的时区，人写「名字 (号)」，没有名字的写号；她自己做的写 `you`。头一行照模板，
//! 一行一条接在后面（模板换字会转义字段里的换行），名字、理由先洗掉控制字符：伪造不出别的行。

use serde_json::{Value, json};

use miyu_im::chat::{Id, To};
use miyu_im::toolset::{Call, NAME, Planned, clean, clock};

use super::args::{Bad, bad_args, group_only, optional_id, optional_number};
use super::moderate::{ADMIN_OFF, REASON};
use crate::admin::records::{self, Action, By, MuteState, Record, Verdict};
use crate::admin::summary::{self, Period, Tally};
use crate::admin::sync::spoken;
use crate::admin::{self, ID};

/// 没写 `limit` 时列几条、几个人。
const SHOWN: usize = 20;

/// `limit` 最多几条（施工 I-17）。
const MOST: usize = 100;

/// 看哪一种。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    /// 一条条记录。
    Events,
    /// 排行榜。
    Ranking(Period),
}

/// 查什么（读好的参数）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Query {
    /// 看哪一种。
    view: View,
    /// 只看这个人。
    user: Option<Id>,
    /// 只看这一种动作（施工 I-17）。
    action: Option<Action>,
    /// 最多几条、几个人（施工 I-17）。
    limit: usize,
}

/// 读参数 `args`（纯逻辑）：写错的交回哪个参数、期望什么。
fn read(args: &Value) -> Result<Query, Bad> {
    let user = optional_id(args, "user", "user must be a QQ number")?;
    let period = match &args["period"] {
        Value::Null => Period::All,
        value => value
            .as_str()
            .and_then(Period::read)
            .ok_or("period must be week, month or all")?,
    };
    let view = match &args["view"] {
        Value::Null => View::Events,
        value => match value.as_str() {
            Some("events") => View::Events,
            Some("ranking") => View::Ranking(period),
            _ => return Err("view must be events or ranking"),
        },
    };
    let action = match &args["action"] {
        Value::Null => None,
        value => Some(match value.as_str() {
            Some("mute") => Action::Mute,
            Some("unmute") => Action::Unmute,
            Some("kick") => Action::Kick,
            _ => return Err("action must be mute, unmute or kick"),
        }),
    };
    let bad = "limit must be 1 to 100";
    let limit = match optional_number(args, "limit", bad)? {
        None => SHOWN,
        Some(limit) => usize::try_from(limit)
            .ok()
            .filter(|limit| *limit <= MOST)
            .ok_or(bad)?,
    };
    Ok(Query {
        view,
        user,
        action,
        limit,
    })
}

/// `moderation_log` 的一次调用（模块的说明）。私聊里调到的答只给群，参数写错的说哪个。
pub(super) fn log(call: Call) -> Planned {
    let To::Group(group) = call.to.clone() else {
        return group_only(&call);
    };
    let query = match read(&call.args) {
        Ok(query) => query,
        Err(bad) => return bad_args(&call, bad),
    };
    let view = query.view;
    if !call.plugins.contains(&ID) {
        return Planned::Now(call.answers.say(ADMIN_OFF, &[], true));
    }
    if matches!(view, View::Ranking(_)) && !admin::summarizing(&call.config) {
        return Planned::Now(call.answers.say("ranking-off", &[], true));
    }
    Planned::later(async move {
        let Some(link) = &call.link else {
            return call.answers.say("unreachable", &[], true);
        };
        if !matches!(
            admin::admin_here(link.as_ref(), &group, &call.bot).await,
            Ok(true)
        ) {
            return call.answers.say(ADMIN_OFF, &[], true);
        }
        let store = call.store.clone();
        let mut kept = tokio::task::spawn_blocking(move || records::read(&store, group))
            .await
            .unwrap_or_default();
        if let Some(user) = &query.user {
            kept.retain(|record| record.user == *user);
        }
        answered(&call, query, &kept, records::now())
    })
}

/// 答的那一句：记录 `kept`（旧的在前，筛过人），此刻是 `now`。
fn answered(call: &Call, query: Query, kept: &[Record], now: u64) -> Value {
    let view = query.view;
    // 禁言的状态照筛动作以前的算（解禁、踢出、后一次禁言都要看）。
    let wanted: Vec<usize> = (0..kept.len())
        .filter(|index| {
            query
                .action
                .is_none_or(|action| kept[*index].action == action)
        })
        .collect();
    let lines: Vec<String> = match view {
        View::Events => wanted
            .iter()
            .rev()
            .take(query.limit)
            .map(|index| {
                let line = event(&kept[*index]);
                match records::mute_state(kept, *index, now) {
                    Some(state) => format!("{line}; status: {}", status(state)),
                    None => line,
                }
            })
            .collect(),
        View::Ranking(period) => summary::ranking(kept, period, now)
            .iter()
            .take(query.limit)
            .enumerate()
            .map(|(index, tally)| ranked(index + 1, tally))
            .collect(),
    };
    if lines.is_empty() {
        return call.answers.say("log-empty", &[], false);
    }
    let head = match view {
        View::Events => {
            let (shown, total) = (lines.len().to_string(), wanted.len().to_string());
            let fields = [("shown", shown.as_str()), ("total", &total)];
            call.answers.text("log-events", &fields)
        }
        View::Ranking(period) => call
            .answers
            .text("log-ranking", &[("period", period.name())]),
    };
    // 模板换字会把字段里的换行转义掉：一行一条接在头一行后面，行里外面来的字（名字、理由）先洗掉控制字符。
    let text = format!("{}\n{}\n", head.trim_end(), lines.join("\n"));
    json!({"blocks": [{"type": "text", "text": text}], "error": false})
}

/// 一个人：「名字 (号)」，没有名字的（洗完空了的）写号。
fn person(name: Option<&str>, user: Id) -> String {
    match name.and_then(|name| clean(name, NAME)) {
        Some(name) => format!("{name} ({user})"),
        None => user.to_string(),
    }
}

/// 禁言的状态怎么写（施工 I-17）。
fn status(state: MuteState) -> &'static str {
    match state {
        MuteState::Muted => "muted now",
        MuteState::Expired => "expired",
        MuteState::Lifted => "unmuted",
        MuteState::Overridden => "overridden by a later mute",
    }
}

/// 几次：`1 time`、`3 times`。
fn times(count: u32) -> String {
    match count {
        1 => "1 time".to_string(),
        count => format!("{count} times"),
    }
}

/// 一条记录一行（纯逻辑）：`2026-10-20 14:03 muted 小明 (40001) for 10 minutes; by you; reason: 刷屏`。
pub(super) fn event(record: &Record) -> String {
    let who = person(record.name.as_deref(), record.user.clone());
    let what = match (record.action, record.seconds) {
        (Action::Mute, Some(seconds)) => format!("muted {who} for {}", spoken(seconds)),
        (Action::Mute, None) => format!("muted {who}"),
        (Action::Unmute, _) => format!("unmuted {who}"),
        (Action::Kick, _) if record.block => format!("removed {who}, blocked from joining again"),
        (Action::Kick, _) => format!("removed {who}"),
        (Action::Join, _) => match record.verdict {
            Some(Verdict::Approve) => format!("approved the join request from {who}"),
            Some(Verdict::Reject) => format!("rejected the join request from {who}"),
            _ => format!("left the join request from {who} to a person"),
        },
    };
    let by = match &record.by {
        By::Ai => "you".to_string(),
        By::Admin { user, name } => person(name.as_deref(), user.clone()),
        By::Spam => "the flood rule".to_string(),
    };
    let mut line = format!("{} {what}; by {by}", clock(record.at));
    if let Some(reason) = record
        .reason
        .as_deref()
        .and_then(|reason| clean(reason, REASON))
    {
        line.push_str("; reason: ");
        line.push_str(&reason);
    }
    line
}

/// 排行榜的一行（纯逻辑）：`1. 小明 (40001): muted 3 times for 1 hour; removed 1 time; last reason: 刷屏`，0 的那一段不写。
pub(super) fn ranked(place: usize, tally: &Tally) -> String {
    let mut parts = Vec::new();
    if tally.mutes > 0 {
        parts.push(format!(
            "muted {} for {}",
            times(tally.mutes),
            spoken(tally.seconds)
        ));
    }
    if tally.kicks > 0 {
        parts.push(format!("removed {}", times(tally.kicks)));
    }
    if let Some(reason) = tally
        .reason
        .as_deref()
        .and_then(|reason| clean(reason, REASON))
    {
        parts.push(format!("last reason: {reason}"));
    }
    let who = person(tally.name.as_deref(), tally.user.clone());
    format!("{place}. {who}: {}", parts.join("; "))
}

#[cfg(test)]
mod tests;
