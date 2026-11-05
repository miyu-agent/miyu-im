//! 判额度（设计 08 第二节第 1、3 条，施工 I-135）：进站链要的「这个会话、发的人满了没有」（[`miyu_chat::Quota`]）在这里判好。
//!
//! - 会话：卡片上的三条线（规则键 `limit_5h`、`limit_week`、`limit_month`，[`crate::limits::KEYS`]），用量照核心的用量账本
//!   （`usage.query {group: ["venue"], from}`，一条线一次，桥自己的连接就能调）。查到的在内存里留一会儿（`bridge.json` 的 `usage_refresh_seconds`，出厂 60 秒），过期了另起任务在
//!   后台再查，这一条先照上一份判；一次都还没查到、查失败的当没满，记一行 WARN：账本坏了不能把所有群都拦住。没有卡片设了线的
//!   不查。
//! - 人：使用限制页上的三条线（im 配置目录里的 `limits.toml`，[`crate::limits::Watched`]），用量照 im 自己的账本
//!   （`crate::spent`，I-134 记的）。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use miyu_chat::Quota;
use miyu_client::ids::ExternalId;
use miyu_client::time::Timestamp;
use serde_json::{Value, json};

use super::charged::{Charging, now};
use crate::TARGET;
use crate::core::Caller;
use crate::limits::{Lines, WINDOWS, Watched};
use crate::spent::Spent;

/// 判额度、记账的一头。
pub(super) struct Limits {
    /// 每人的用量记账（I-134）。
    pub(super) charging: Charging,
    /// 每人的用量账本。
    spent: Arc<Spent>,
    /// 每人的设置。
    people: Mutex<Watched>,
    /// 各会话的用量，照核心的账本查来的。
    venues: Arc<Mutex<Totals>>,
    /// 查核心的调用口。
    caller: Caller,
    /// 会话的用量查到的留多久（`bridge.json` 的 `usage_refresh_seconds`）。
    fresh: Duration,
}

/// 各会话的用量。
#[derive(Debug, Default)]
struct Totals {
    /// 上一次查到的时刻；还没查到过的是空的。
    at: Option<Instant>,
    /// 正在查。
    busy: bool,
    /// 照 [`WINDOWS`] 的先后：场所编号 → 四项合计。
    used: [HashMap<String, u64>; 3],
}

impl Limits {
    /// 记进 `spent`、每人的设置在 `people`（文件）、会话的用量查到的留 `fresh`、查核心用 `caller`。
    pub(super) fn new(
        spent: Arc<Spent>,
        (people, fresh): (PathBuf, Duration),
        caller: Caller,
    ) -> Limits {
        Limits {
            charging: Charging::new(Arc::clone(&spent)),
            spent,
            people: Mutex::new(Watched::new(people)),
            venues: Arc::new(Mutex::new(Totals::default())),
            caller,
            fresh,
        }
    }

    /// 场所 `venue`（卡片上的线是 `lines`）、发的人 `person` 此刻满了没有。
    pub(super) fn quota(&self, venue: &str, lines: Lines, person: &ExternalId) -> Quota {
        Quota {
            venue: self.venue_full(venue, lines),
            person: self.person_full(person.as_str()),
        }
    }

    /// 场所 `venue` 碰到了卡片上的线 `lines` 没有：没写线的不查。
    pub(super) fn venue_full(&self, venue: &str, lines: Lines) -> bool {
        if lines.is_empty() {
            return false;
        }
        lines.reached(self.venue_used(venue))
    }

    /// `person` 碰到了使用限制页上给他的线没有。
    fn person_full(&self, person: &str) -> bool {
        let lines = self
            .people
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .people()
            .lines_for(person);
        if lines.is_empty() {
            return false;
        }
        let now = now();
        lines.reached(WINDOWS.map(|window| self.spent.since(person, now - window)))
    }

    /// 场所 `venue` 三条线各用了多少，照上一份查到的；过期了另起任务再查。
    fn venue_used(&self, venue: &str) -> [u64; 3] {
        let mut totals = self.venues.lock().unwrap_or_else(PoisonError::into_inner);
        if !totals.busy && totals.at.is_none_or(|at| at.elapsed() >= self.fresh) {
            totals.busy = true;
            tokio::spawn(refresh(self.caller.clone(), Arc::clone(&self.venues)));
        }
        let used = &totals.used;
        [0, 1, 2].map(|line| used[line].get(venue).copied().unwrap_or_default())
    }
}

/// 照核心的账本再查一遍各会话三条线的用量；有一条查不成的这一回不换，照旧用上一份。
async fn refresh(caller: Caller, totals: Arc<Mutex<Totals>>) {
    let now = now();
    let mut used: [HashMap<String, u64>; 3] = Default::default();
    let mut whole = true;
    for (line, window) in WINDOWS.into_iter().enumerate() {
        let Some(from) = Timestamp::from_unix_millis(now - window) else {
            whole = false;
            break;
        };
        let params = json!({"group": ["venue"], "from": from.to_string()});
        match caller.call("usage.query", params).await {
            Ok(reply) if reply["result"]["rows"].is_array() => {
                used[line] = rows(&reply["result"]["rows"]);
            }
            Ok(reply) => {
                tracing::warn!(target: TARGET, error = %reply["error"], "venue usage not queried");
                whole = false;
                break;
            }
            Err(_) => {
                whole = false;
                break;
            }
        }
    }
    let mut totals = totals.lock().unwrap_or_else(PoisonError::into_inner);
    totals.busy = false;
    totals.at = Some(Instant::now());
    if whole {
        totals.used = used;
    }
}

/// `usage.query` 照场所分的几行：场所编号 → 四项合计。没有场所的（一次性调用）不要。
fn rows(rows: &Value) -> HashMap<String, u64> {
    let mut used = HashMap::new();
    for row in rows.as_array().into_iter().flatten() {
        let Some(venue) = row["venue"].as_str() else {
            continue;
        };
        let tokens = ["uncached", "cache_read", "cache_write", "output"]
            .iter()
            .map(|key| row["usage"][key].as_u64().unwrap_or_default())
            .fold(0_u64, u64::saturating_add);
        used.insert(venue.to_string(), tokens);
    }
    used
}

#[cfg(test)]
mod tests;
