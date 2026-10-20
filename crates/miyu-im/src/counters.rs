//! 今天的几个数（设计 01 第八节第 2 条首页「今天」，施工 I-6 下）：收到的（交进场所会话的每一条消息）、回复的（她说的每一段发
//! 出去了）、只记录的（群里判下来只记下的）、没发出去的（她说的一段发不出去、退了信）。
//!
//! - 照本机此刻的日期分天：跨了天的从零数。
//! - 记在 im 自己的状态目录里（`state/packages/im/stats/<日期>.json`，一份一行 JSON），桥起来时读今天那一份接着数；改了另起的
//!   任务照着写（[`keep`]，先写临时文件再改名盖上），写不进记一行运行日志，照数。
//! - 每数一下叫一声 `crate::changed`：开着的页面照它再问（不轮询），状态文件照它比一比、一样的不写。
//!
//! - 每个场所另有一份（施工 I-7 上补：会话浮窗「概况」的今天收到、回复、只记录）：同一个文件里的 `venues`，场所编号 → 几个数。
//!
//! - 统计页（施工 I-14）另记两样：判断的原因（`reasons`，原因的编号 → 几次，同会话浮窗「最近判断」的原因）、收到的按本机的小时
//!   （`hours`，24 格）。[`Counters::days`] 读一段日子的（今天照手里的，别的照盘上的）。

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

use crate::TARGET;
use crate::changed::Changed;
use crate::serve::Failure;

/// 数哪一样。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Count {
    /// 收到的。
    Received,
    /// 回复的。
    Replied,
    /// 只记录的。
    Recorded,
    /// 没发出去的。
    Undelivered,
}

/// 一天的几个数。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Day {
    pub(crate) received: u64,
    pub(crate) replied: u64,
    pub(crate) recorded: u64,
    pub(crate) undelivered: u64,
}

/// 存的一天：合计（写在最外层，同施工 I-6 下的写法）、每个场所的、判断的原因和按小时收到的（施工 I-14）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Daily {
    /// 合计。
    #[serde(flatten)]
    pub(crate) total: Day,
    /// 场所编号 → 几个数。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) venues: BTreeMap<String, Day>,
    /// 判断的原因 → 几次（施工 I-14）。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) reasons: BTreeMap<String, u64>,
    /// 收到的，按本机的小时（0 到 23 点，施工 I-14）；以前写的没有这一格。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) hours: Vec<u64>,
}

impl Day {
    /// 数一下 `count`。
    fn add(&mut self, count: Count) {
        match count {
            Count::Received => self.received += 1,
            Count::Replied => self.replied += 1,
            Count::Recorded => self.recorded += 1,
            Count::Undelivered => self.undelivered += 1,
        }
    }

    /// 加上 `other`。
    fn plus(self, other: Day) -> Day {
        Day {
            received: self.received + other.received,
            replied: self.replied + other.replied,
            recorded: self.recorded + other.recorded,
            undelivered: self.undelivered + other.undelivered,
        }
    }
}

/// 今天的几个数，几处共用。
pub(crate) struct Counters {
    /// 每天一份的目录。
    dir: PathBuf,
    /// 哪一天、那一天的数。
    held: Mutex<(String, Daily)>,
    /// 变了叫一声（页面、状态文件）。
    changed: Arc<Changed>,
    /// 变了叫写盘的那个任务。
    dirty: Notify,
}

impl Counters {
    /// 照目录 `dir` 读今天那一份（没有的、读不懂的从零数），变了叫 `changed`。
    pub(crate) fn load(dir: PathBuf, changed: Arc<Changed>) -> Counters {
        let date = today();
        let day = std::fs::read(file(&dir, &date))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Counters {
            dir,
            held: Mutex::new((date, day)),
            changed,
            dirty: Notify::new(),
        }
    }

    /// 场所 `venue` 上数一下 `count`（合计也数）。
    pub(crate) fn bump(&self, count: Count, venue: &str) {
        self.bump_on(count, venue, &today());
    }

    /// 同 [`Counters::bump`]，今天是 `date`（测试照它跨天）。
    fn bump_on(&self, count: Count, venue: &str, date: &str) {
        {
            let mut held = lock(&self.held);
            if held.0 != date {
                *held = (date.to_string(), Daily::default());
            }
            let daily = &mut held.1;
            daily.total.add(count);
            if count == Count::Received {
                hour(daily, now_hour());
            }
            daily
                .venues
                .entry(venue.to_string())
                .or_default()
                .add(count);
        }
        self.dirty.notify_one();
        self.changed.notify();
    }

    /// 判断了一次，原因是 `reason`（施工 I-14，统计页「为什么回、为什么没回」）。
    pub(crate) fn reason(&self, reason: &str) {
        {
            let date = today();
            let mut held = lock(&self.held);
            if held.0 != date {
                *held = (date, Daily::default());
            }
            *held.1.reasons.entry(reason.to_string()).or_default() += 1;
        }
        self.dirty.notify_one();
    }

    /// 日期 `dates` 那几天的（施工 I-14，统计页）：今天照手里的，别的照盘上的；没有的、读不懂的当零。
    pub(crate) fn days(&self, dates: &[String]) -> Vec<(String, Daily)> {
        let held = lock(&self.held).clone();
        dates
            .iter()
            .map(|date| {
                let daily = if *date == held.0 {
                    held.1.clone()
                } else {
                    std::fs::read(file(&self.dir, date))
                        .ok()
                        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                        .unwrap_or_default()
                };
                (date.clone(), daily)
            })
            .collect()
    }

    /// 今天的几个数（记着的是前一天的当零）。
    pub(crate) fn today(&self) -> Day {
        self.on(&today())
    }

    /// 今天合 `wanted` 的几个场所的数加起来（施工 I-7 上补：「陌生群」是名单以外的群并起来）。
    pub(crate) fn today_of(&self, wanted: impl Fn(&str) -> bool) -> Day {
        self.of_on(wanted, &today())
    }

    /// 同 [`Counters::today`]，今天是 `date`。
    fn on(&self, date: &str) -> Day {
        let held = lock(&self.held);
        if held.0 == date {
            held.1.total
        } else {
            Day::default()
        }
    }

    /// 同 [`Counters::today_of`]，今天是 `date`。
    fn of_on(&self, wanted: impl Fn(&str) -> bool, date: &str) -> Day {
        let held = lock(&self.held);
        if held.0 != date {
            return Day::default();
        }
        held.1
            .venues
            .iter()
            .filter(|(venue, _)| wanted(venue))
            .fold(Day::default(), |sum, (_, day)| sum.plus(*day))
    }
}

/// 改了就写（模块的说明）：一直写下去，桥停下时随一组任务一起停；交回的类型照那一组（`crate::serve`）。
pub(crate) async fn keep(counters: Arc<Counters>) -> Option<Failure> {
    loop {
        counters.dirty.notified().await;
        let (date, day) = lock(&counters.held).clone();
        let path = file(&counters.dir, &date);
        let bytes = match serde_json::to_vec(&day) {
            Ok(mut bytes) => {
                bytes.push(b'\n');
                bytes
            }
            Err(error) => {
                tracing::warn!(target: TARGET, error = %error, "counters not written");
                continue;
            }
        };
        let target = path.clone();
        match tokio::task::spawn_blocking(move || miyu_client::places::write(&target, &bytes)).await
        {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                tracing::warn!(target: TARGET, file = %path.display(), error = %error, "counters not written");
            }
            Err(error) => tracing::warn!(target: TARGET, error = %error, "counters not written"),
        }
    }
}

/// 第 `hour` 点收到的那一格数一下：以前写的没有 24 格的先补齐。
fn hour(daily: &mut Daily, hour: usize) {
    if daily.hours.len() != 24 {
        daily.hours.resize(24, 0);
    }
    if let Some(slot) = daily.hours.get_mut(hour) {
        *slot += 1;
    }
}

/// 本机此刻是几点（0 到 23）。
fn now_hour() -> usize {
    usize::try_from(jiff::Zoned::now().hour()).unwrap_or_default()
}

/// 日期 `date` 那一份的路径。
fn file(dir: &Path, date: &str) -> PathBuf {
    dir.join(format!("{date}.json"))
}

/// 本机此刻的日期：`2026-10-19` 这样。
fn today() -> String {
    jiff::Zoned::now().date().to_string()
}

/// 锁里不 `await`、不会崩；真崩了，里面的照样能用。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
