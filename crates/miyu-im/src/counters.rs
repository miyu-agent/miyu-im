//! 今天的几个数（设计 01 第八节第 2 条首页「今天」，施工 I-6 下）：收到的（交进场所会话的每一条消息）、回复的（她说的每一段发
//! 出去了）、只记录的（群里判下来只记下的）、没发出去的（她说的一段发不出去、退了信）。
//!
//! - 照本机此刻的日期分天：跨了天的从零数。
//! - 记在 im 自己的状态目录里（`state/packages/im/stats/<日期>.json`，一份一行 JSON），桥起来时读今天那一份接着数；改了另起的
//!   任务照着写（[`keep`]，先写临时文件再改名盖上），写不进记一行运行日志，照数。
//! - 每数一下叫一声 `crate::changed`：开着的页面照它再问（不轮询），状态文件照它比一比、一样的不写。
//!
//! 统计页要的分会话、分判断结果、分小时随统计那一步加。

#[cfg(test)]
mod tests;

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

/// 今天的几个数，几处共用。
pub(crate) struct Counters {
    /// 每天一份的目录。
    dir: PathBuf,
    /// 哪一天、那一天的数。
    held: Mutex<(String, Day)>,
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

    /// 数一下 `count`。
    pub(crate) fn bump(&self, count: Count) {
        self.bump_on(count, &today());
    }

    /// 同 [`Counters::bump`]，今天是 `date`（测试照它跨天）。
    fn bump_on(&self, count: Count, date: &str) {
        {
            let mut held = lock(&self.held);
            if held.0 != date {
                *held = (date.to_string(), Day::default());
            }
            let day = &mut held.1;
            match count {
                Count::Received => day.received += 1,
                Count::Replied => day.replied += 1,
                Count::Recorded => day.recorded += 1,
                Count::Undelivered => day.undelivered += 1,
            }
        }
        self.dirty.notify_one();
        self.changed.notify();
    }

    /// 今天的几个数（记着的是前一天的当零）。
    pub(crate) fn today(&self) -> Day {
        self.on(&today())
    }

    /// 同 [`Counters::today`]，今天是 `date`。
    fn on(&self, date: &str) -> Day {
        let held = lock(&self.held);
        if held.0 == date {
            held.1
        } else {
            Day::default()
        }
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
