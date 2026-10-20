//! 查管理记录（施工 Q-4，设计 02 第一节第 1、4 条）：`moderation_log {view?, user?, period?}`，只给群。`events` 是这个群的记录，新的
//! 在前，最多 [`SHOWN`] 条，写了 `user` 的只看这个人；`ranking` 是成员汇总排的榜（`period`：`week`、`month`、`all`，出厂 `all`），写了
//! `user` 的只看这个人那一行（成员汇总关了的答 `ranking-off`）。群管理在这个群生效才查（插件开着、她是管理员或群主）。
//!
//! 每一行的样子在这里拼（同群成员列表）：时刻照本机的时区，人写「名字 (号)」，没有名字的写号；她自己做的写 `you`。头一行照模板，
//! 一行一条接在后面（模板换字会转义字段里的换行），名字、理由先洗掉控制字符：伪造不出别的行。

use serde_json::{Value, json};

use miyu_im::chat::To;
use miyu_im::toolset::{Call, NAME, Planned, clean, clock};

use super::moderate::{ADMIN_OFF, REASON};
use crate::admin::records::{self, Action, By, Record, Verdict};
use crate::admin::summary::{self, Period, Tally};
use crate::admin::sync::spoken;
use crate::admin::{self, ID};

/// 一次最多列几条、几个人。
const SHOWN: usize = 20;

/// 看哪一种。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    /// 一条条记录。
    Events,
    /// 排行榜。
    Ranking(Period),
}

/// 读参数 `args`：看哪一种、只看谁。写错的是空的。
fn read(args: &Value) -> Option<(View, Option<i64>)> {
    let user = match &args["user"] {
        Value::Null => None,
        value => Some(value.as_i64().filter(|user| *user > 0)?),
    };
    let period = match &args["period"] {
        Value::Null => Period::All,
        value => Period::read(value.as_str()?)?,
    };
    let view = match args["view"].as_str() {
        None | Some("events") => View::Events,
        Some("ranking") => View::Ranking(period),
        Some(_) => return None,
    };
    Some((view, user))
}

/// `moderation_log` 的一次调用（模块的说明）。参数写错的、私聊里调到的照不认识的工具答。
pub(super) fn log(call: Call) -> Planned {
    let (To::Group(group), Some((view, user))) = (call.to, read(&call.args)) else {
        return Planned::Now(call.answers.say("unknown", &[("name", &call.tool)], true));
    };
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
            admin::admin_here(link.as_ref(), group, call.bot).await,
            Ok(true)
        ) {
            return call.answers.say(ADMIN_OFF, &[], true);
        }
        let store = call.store.clone();
        let mut kept = tokio::task::spawn_blocking(move || records::read(&store, group))
            .await
            .unwrap_or_default();
        if let Some(user) = user {
            kept.retain(|record| record.user == user);
        }
        answered(&call, view, &kept, records::now())
    })
}

/// 答的那一句：记录 `kept`（旧的在前，筛过人），此刻是 `now`。
fn answered(call: &Call, view: View, kept: &[Record], now: u64) -> Value {
    let lines: Vec<String> = match view {
        View::Events => kept.iter().rev().take(SHOWN).map(event).collect(),
        View::Ranking(period) => summary::ranking(kept, period, now)
            .iter()
            .take(SHOWN)
            .enumerate()
            .map(|(index, tally)| ranked(index + 1, tally))
            .collect(),
    };
    if lines.is_empty() {
        return call.answers.say("log-empty", &[], false);
    }
    let head = match view {
        View::Events => {
            let (shown, total) = (lines.len().to_string(), kept.len().to_string());
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
fn person(name: Option<&str>, user: i64) -> String {
    match name.and_then(|name| clean(name, NAME)) {
        Some(name) => format!("{name} ({user})"),
        None => user.to_string(),
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
    let who = person(record.name.as_deref(), record.user);
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
        By::Admin { user, name } => person(name.as_deref(), *user),
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
    let who = person(tally.name.as_deref(), tally.user);
    format!("{place}. {who}: {}", parts.join("; "))
}

#[cfg(test)]
mod tests;
