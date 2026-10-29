//! 群管理按群的设置（施工 Q-5，设计 02 第四节第 4 条：在会话浮窗的「群管理」页里改，施工 Q-6）：`<驱动的状态目录>/admin/groups.json`，
//! `{"<群号>": {"spam": {"on", "window", "mute"}, "view": {"action", "period"}, "clearing"?}}`（面板上怎么看、点没点清空，施工 Q-6）。没写的、读不了的照出厂的：刷屏处理关着（2026-10-20 项目主人定），时间窗 60 秒，
//! 禁言 10 分钟。读的一方照文件的修改时刻判断要不要重读，不每条消息都读盘。

use miyu_im::chat::Id;
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

/// 面板上这个群的记录怎么看（施工 Q-6）：哪一种动作（`all`、`mute`、`unmute`、`kick`）、排行榜看多久（`week`、`month`、`all`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct View {
    /// 哪一种动作。
    pub action: String,
    /// 排行榜看多久。
    pub period: String,
}

impl Default for View {
    fn default() -> View {
        View {
            action: "all".to_string(),
            period: "week".to_string(),
        }
    }
}

/// 一个群的入群审批（施工 Q-7）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Approval {
    /// 通过条件：项目主人写的，照原样进提示词；空的不审，留给人工。
    pub rule: String,
}

/// 一个群的设置。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Group {
    /// 刷屏处理。
    pub spam: Spam,
    /// 入群审批（施工 Q-7）。
    pub approval: Approval,
    /// 面板上怎么看（施工 Q-6）。
    pub view: View,
    /// 点了「清空记录」的时刻（Unix 秒，施工 Q-6）：一分钟以内再点一次才清。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clearing: Option<u64>,
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

/// 改群 `group` 的设置：读、交给 `change` 改、写到旁边再换上（同一个进程里照一把锁排着）。
pub(crate) fn update(
    store: &Path,
    group: Id,
    change: impl FnOnce(&mut Group),
) -> std::io::Result<()> {
    let _writing = WRITING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut groups = read(store);
    change(groups.entry(group.to_string()).or_default());
    let path = file(store);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let partial = path.with_extension("json.partial");
    std::fs::write(
        &partial,
        serde_json::to_vec_pretty(&groups).map_err(std::io::Error::other)?,
    )?;
    std::fs::rename(&partial, &path)
}

/// 同一个进程里排着改。
static WRITING: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    pub(crate) fn group(&mut self, store: &Path, group: Id) -> Group {
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
