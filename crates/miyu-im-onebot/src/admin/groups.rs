//! 群管理按群的设置（施工 Q-5，设计 02 第四节第 4 条：在会话浮窗的「群管理」页里改，施工 Q-6）：`<驱动的状态目录>/admin/groups.json`，
//! `{"<群号>": {"spam": {"on", "window", "mute"}}}`。没写的、读不了的照出厂的：刷屏处理关着（2026-10-20 项目主人定），时间窗 60 秒，
//! 禁言 10 分钟。读的一方照文件的修改时刻判断要不要重读，不每条消息都读盘。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// 刷屏处理出厂的时间窗：秒。
const WINDOW: u64 = 60;

/// 刷屏处理出厂的禁言时长：秒。
const MUTE: u64 = 600;

/// 一个群的刷屏处理。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Spam {
    /// 开着没有。
    pub on: bool,
    /// 连着发几次要在多少秒以内才算。
    pub window: u64,
    /// 禁言几秒。
    pub mute: u64,
}

impl Default for Spam {
    fn default() -> Spam {
        Spam {
            on: false,
            window: WINDOW,
            mute: MUTE,
        }
    }
}

/// 一个群的设置。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Group {
    /// 刷屏处理。
    pub spam: Spam,
}

/// 按群的设置文件在哪：驱动的状态目录 `store` 下。
pub(crate) fn file(store: &Path) -> PathBuf {
    store.join("admin").join("groups.json")
}

/// 读按群的设置；没有的、读不了的、写坏了的是空的（都照出厂的）。
pub(crate) fn read(store: &Path) -> HashMap<String, Group> {
    std::fs::read_to_string(file(store))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// 读着的一份：文件的修改时刻变了才重读。
#[derive(Debug, Default)]
pub(crate) struct Cached {
    /// 读的时候文件的修改时刻；没有文件的是空的。
    stamp: Option<SystemTime>,
    /// 读出来的。
    groups: HashMap<String, Group>,
    /// 读过没有。
    loaded: bool,
}

impl Cached {
    /// 群 `group` 的设置：文件变了先重读。
    pub(crate) fn group(&mut self, store: &Path, group: i64) -> Group {
        let stamp = std::fs::metadata(file(store))
            .and_then(|meta| meta.modified())
            .ok();
        if !self.loaded || stamp != self.stamp {
            self.groups = read(store);
            self.stamp = stamp;
            self.loaded = true;
        }
        self.groups
            .get(&group.to_string())
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests;
