//! 管理记录（施工 Q-2，设计 02 第一节第 2 条、第四节）：一个群一份 `<驱动的状态目录>/admin/<群号>.jsonl`，一行一条，照先后追加；
//! 超过「每个群留多少条」（`admin_keep`）的从旧的删。面板、浮窗、查记录的工具都读这一份（施工 Q-4 起）。
//!
//! 写一条是一次读改写（超了要删旧的）：同一个进程里照一把锁排着写，另起阻塞的任务做，不卡跟核心的那一头。写坏了的行读的时候跳过。

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// 同一个进程里排着写（读改写不交叉）。
static WRITING: Mutex<()> = Mutex::new(());

/// 一条管理记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Record {
    /// 什么时候（本机此刻的 Unix 秒）。
    pub at: u64,
    /// 做了什么。
    pub action: Action,
    /// 对谁：号。
    pub user: i64,
    /// 对谁：那时在群里叫什么（洗过）；不知道的是空的。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 谁做的。
    pub by: By,
    /// 禁言几秒（禁言才有）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u64>,
    /// 理由（AI 给的一句，洗过）；没给的是空的。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// 踢人时以后不收他的加群申请。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub block: bool,
}

/// 做了什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Action {
    /// 禁言。
    Mute,
    /// 解禁。
    Unmute,
    /// 移出群。
    Kick,
}

/// 谁做的。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum By {
    /// AI 用工具做的。
    Ai,
    /// 群里别的管理员（施工 Q-3：平台的通知同步进来）。
    Admin {
        /// 号。
        user: i64,
        /// 那时在群里叫什么；不知道的是空的。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// 刷屏处理（施工 Q-5）。
    Spam,
}

/// 本机此刻的 Unix 秒；钟在 1970 年以前的是 0。
pub(crate) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// 群 `group` 的记录放在哪：驱动的状态目录 `store` 下。
pub(crate) fn file(store: &Path, group: i64) -> PathBuf {
    store.join("admin").join(format!("{group}.jsonl"))
}

/// 给群 `group` 记一条 `record`，最多留 `keep` 条（超了从旧的删）。写不进去的记一行运行日志，不交回错：记录写不进不影响动作本身。
pub(crate) async fn append(store: PathBuf, group: i64, record: Record, keep: usize) {
    let path = file(&store, group);
    let written = tokio::task::spawn_blocking(move || write(&path, &record, keep)).await;
    if let Ok(Err(error)) | Err(error) = written.map_err(std::io::Error::other) {
        tracing::warn!(target: miyu_im::TARGET, group, error = %error, "moderation record not written");
    }
}

/// 读改写：原来的几行（写坏了的跳过）加上这一条，只留最后 `keep` 条，写到旁边再换上。
fn write(path: &Path, record: &Record, keep: usize) -> std::io::Result<()> {
    let _writing = WRITING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut lines: Vec<String> = std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|line| serde_json::from_str::<Record>(line).is_ok())
        .map(str::to_string)
        .collect();
    lines.push(serde_json::to_string(record).map_err(std::io::Error::other)?);
    let start = lines.len().saturating_sub(keep);
    let mut text = lines[start..].join("\n");
    text.push('\n');
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let partial = path.with_extension("jsonl.partial");
    std::fs::write(&partial, text)?;
    std::fs::rename(&partial, path)
}

#[cfg(test)]
mod tests;
