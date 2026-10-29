//! 最近判断（设计 01 第八节第 7 条，会话浮窗「概况」的「最近判断」；施工 I-7 上补）：群里每判一条，记一笔回没回、为什么，每个会话
//! 留最近的 [`MOST`] 笔。
//!
//! - 一笔（[`Note`]）：时刻、发的人的名字、正文的开头（最多 [`TEXT`] 个字）、结论（`reply`、`record`、`deferred`）、为什么（照判断
//!   的记录说成一个原因，`crate::core::route` 的 `noted`）。
//! - 记在 im 自己的状态目录里（`state/packages/miyu-im/recent.json`，一份 JSON：场所编号 → 一笔笔，旧的在前；不放进 `stats/`，那里一天
//!   一份），桥起来时读回来接着记；
//!   改了另起的任务照着写（[`keep`]，同 `crate::counters`），写不进的记一行运行日志，照记。
//! - 读（[`Recent::of`]）：几个场所的并在一起，新的在前，最多 [`MOST`] 笔（「陌生群」是名单以外的群并起来）。

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Notify;

use crate::TARGET;
use crate::serve::Failure;

/// 每个会话留几笔，读的时候最多交几笔。
pub(crate) const MOST: usize = 20;

/// 正文最多留几个字。
pub(crate) const TEXT: usize = 80;

/// 一笔判断。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Note {
    /// 什么时候判的：毫秒的 Unix 时间。
    pub(crate) at: u64,
    /// 发的人的名字（没有的是号）。
    pub(crate) who: String,
    /// 正文的开头。
    pub(crate) text: String,
    /// 结论：`reply`、`record`、`deferred`。
    pub(crate) outcome: String,
    /// 为什么：`{"reason", ...}`（`noted` 说的）。
    pub(crate) why: Value,
}

impl Note {
    /// 发的人写成什么：名字（群名片、昵称）去掉两头的空白；没有的、空白的写平台上的身份 `fallback`。
    pub(crate) fn who(name: Option<&str>, fallback: &str) -> String {
        name.map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(fallback)
            .to_string()
    }

    /// 正文 `text` 留开头 [`TEXT`] 个字，多的写省略号。
    pub(crate) fn excerpt(text: &str) -> String {
        let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if flat.chars().count() <= TEXT {
            return flat;
        }
        let mut cut: String = flat.chars().take(TEXT).collect();
        cut.push('…');
        cut
    }
}

/// 最近判断，几处共用。
pub(crate) struct Recent {
    /// 存的那一份。
    file: PathBuf,
    /// 场所编号 → 一笔笔，旧的在前。
    held: Mutex<BTreeMap<String, VecDeque<Note>>>,
    /// 变了叫写盘的那个任务。
    dirty: Notify,
}

impl Recent {
    /// 照文件 `file` 读回来（没有的、读不懂的从空的记）。
    pub(crate) fn load(file: PathBuf) -> Recent {
        let held = std::fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Recent {
            file,
            held: Mutex::new(held),
            dirty: Notify::new(),
        }
    }

    /// 场所 `venue` 记一笔：超过 [`MOST`] 笔的去掉最旧的。
    pub(crate) fn note(&self, venue: &str, note: Note) {
        {
            let mut held = lock(&self.held);
            let notes = held.entry(venue.to_string()).or_default();
            notes.push_back(note);
            while notes.len() > MOST {
                notes.pop_front();
            }
        }
        self.dirty.notify_one();
    }

    /// 合 `wanted` 的几个场所的，新的在前，最多 [`MOST`] 笔；每笔带上它的场所编号（`venue`）。
    pub(crate) fn of(&self, wanted: impl Fn(&str) -> bool) -> Vec<(String, Note)> {
        let held = lock(&self.held);
        let mut all: Vec<(String, Note)> = held
            .iter()
            .filter(|(venue, _)| wanted(venue))
            .flat_map(|(venue, notes)| notes.iter().map(|note| (venue.clone(), note.clone())))
            .collect();
        all.sort_by_key(|(_, note)| std::cmp::Reverse(note.at));
        all.truncate(MOST);
        all
    }
}

/// 改了就写（模块的说明）：一直写下去，桥停下时随一组任务一起停；交回的类型照那一组（`crate::serve`）。
pub(crate) async fn keep(recent: Arc<Recent>) -> Option<Failure> {
    loop {
        recent.dirty.notified().await;
        let bytes = match serde_json::to_vec(&*lock(&recent.held)) {
            Ok(mut bytes) => {
                bytes.push(b'\n');
                bytes
            }
            Err(error) => {
                tracing::warn!(target: TARGET, error = %error, "recent decisions not written");
                continue;
            }
        };
        let target = recent.file.clone();
        match tokio::task::spawn_blocking(move || miyu_client::places::write(&target, &bytes)).await
        {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                tracing::warn!(target: TARGET, file = %recent.file.display(), error = %error, "recent decisions not written");
            }
            Err(error) => {
                tracing::warn!(target: TARGET, error = %error, "recent decisions not written")
            }
        }
    }
}

/// 此刻：毫秒的 Unix 时间（早于 1970 年的是 0）。
pub(crate) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// 锁里不 `await`、不会崩；真崩了，里面的照样能用。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
