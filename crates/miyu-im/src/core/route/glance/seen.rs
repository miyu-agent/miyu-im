//! 看一眼看得见（施工 I-114，项目主人 11-02：开了以后不知道有没有生效）：每个场所记上一次几点看的、看到几条新消息，下一次几点、是她定的
//! （`qq_glance`）还是照间隔排的。会话浮窗「概况」照 [`Route::glance_seen`] 写；首页「活动」照运行日志的 `glance opened`、
//! `glance armed`（带 `by`）写。她没定下一次时的间隔照规则的 `glance_minutes`（卡上「多久看一眼」），没写的照出厂
//! （[`Route::glance_every`]）。桥重启了从头记（定时在核心那边，照旧到点）。

use miyu_client::time::Timestamp;
use serde_json::{Value, json};

use super::super::{Route, applied};
use super::{later, parse};

/// 一个场所的：下一次（几点、是不是她定的），上一次（几点、看到几条新消息）。
#[derive(Debug, Default, Clone)]
pub(in crate::core::route) struct Seen {
    next: Option<(Timestamp, bool)>,
    last: Option<(Timestamp, usize)>,
}

impl Route {
    /// 场所 `venue` 她没定下一次时隔几秒：规则的 `glance_minutes`（照她定的一分钟按几秒算），没写的照出厂。
    pub(in crate::core::route) fn glance_every(&mut self, venue: &str) -> u64 {
        let minutes = parse(venue).and_then(|venue| applied::glance_minutes(&self.applied(&venue)));
        minutes.map_or(self.glances.default.as_secs(), |minutes| {
            minutes.saturating_mul(self.glances.minute)
        })
    }

    /// 场所 `venue` 排上了下一次：`at` 几点，`hers` 是不是她定的。
    pub(in crate::core::route) fn glance_planned(
        &mut self,
        venue: &str,
        at: Timestamp,
        hers: bool,
    ) {
        self.glances.seen.entry(venue.to_string()).or_default().next = Some((at, hers));
    }

    /// 场所 `venue` 刚看了一眼，看到 `count` 条新消息（私聊的不数，是 0）。
    pub(in crate::core::route) fn glance_looked(&mut self, venue: &str, count: usize) {
        let seen = self.glances.seen.entry(venue.to_string()).or_default();
        seen.next = None;
        seen.last = later(0).map(|now| (now, count));
    }

    /// 一张卡管的场所（`wanted` 认）里最近看过、或者排着的那一个：`{next?, hers?, last?, count?}`，都没有的是空的。
    pub(in crate::core::route) fn glance_seen(&self, wanted: &dyn Fn(&str) -> bool) -> Value {
        let mut shown: Option<&Seen> = None;
        for (venue, seen) in &self.glances.seen {
            if !wanted(venue) {
                continue;
            }
            let armed = self.glances.armed.contains_key(venue);
            let next = if armed { seen.next } else { None };
            if next.is_none() && seen.last.is_none() {
                continue;
            }
            let newer = |one: &Seen| one.last.map(|(at, _)| at);
            if shown.is_none_or(|before| newer(seen) > newer(before)) {
                shown = Some(seen);
            }
        }
        let Some(seen) = shown else {
            return Value::Null;
        };
        let mut found = json!({});
        if let Some((at, hers)) = seen.next {
            found["next"] = json!(at.to_string());
            found["hers"] = json!(hers);
        }
        if let Some((at, count)) = seen.last {
            found["last"] = json!(at.to_string());
            found["count"] = json!(count);
        }
        found
    }
}
