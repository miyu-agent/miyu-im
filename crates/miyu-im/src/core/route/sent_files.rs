//! 她发出去的文件（施工 I-247，2026-11-17 项目主人验收第 35 条）：私聊里对方下载了她传的文件，QQ 那边出一条灰色的「你已成功
//! 下载文件」，NapCat 推来的却是一条普通的私聊消息：发的人是对方，只有一个文件段，就是她传的那个。原来照对方发了一个文件
//! 开一轮，她就回了一句。
//!
//! - 她传成了的文件（`qq_send_file`、`show` 传成文件的）按场所记下名字、字节数、平台回的文件编号（QQ 的 `file_id`，施工 I-268）
//!   和时刻：每个场所最近 [`PER_VENUE`] 份，[`KEEP`] 以后作废。
//! - 记在 im 自己的状态目录里（[`FILE`]，施工 I-268）：原来只在内存里、24 小时作废，11-20 主人下载了两天前她发的三个文件，中间
//!   桥还重启过，三条都开了一轮。每记一份写一次（文件很小，传文件本来就少）；写不进的记一行运行日志，照记在内存里。
//! - 一条消息没有字、没有 @、没有引用和转发，只带一个文件（[`receipt`]）：文件编号对得上这个场所记着的一份，或者名字、字节数都
//!   对得上，是下载的回执。平台没回文件编号的（Telegram、旧的 NapCat）只能照名字、字节数认。

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::TARGET;
use crate::chat::{MediaKind, Piece, Segments};

/// 状态目录里的文件名。
pub(super) const FILE: &str = "sent-files.json";

/// 记多久：毫秒（30 天）。
const KEEP: u64 = 30 * 24 * 60 * 60 * 1000;

/// 每个场所最多记几份。
const PER_VENUE: usize = 64;

/// 场所 → 她传成了的文件，照先后。跟核心的那一头和传文件的任务共用一份。
#[derive(Debug, Clone, Default)]
pub(super) struct SentFiles(Arc<Held>);

/// 记着的和写到哪（测试里造的不写盘）。
#[derive(Debug, Default)]
struct Held {
    file: Option<PathBuf>,
    venues: Mutex<BTreeMap<String, VecDeque<Sent>>>,
}

/// 传成了的一份。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct Sent {
    /// 文件名。
    pub(super) name: String,
    /// 字节数。
    pub(super) size: u64,
    /// 平台回的文件编号，没回的是空的。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) id: Option<String>,
    /// 什么时候：毫秒的 Unix 时间。
    pub(super) at: u64,
}

/// 收到的一个文件：平台编号、名字、字节数。
pub(super) type Received<'a> = (&'a str, &'a str, u64);

impl SentFiles {
    /// 照文件 `file` 读回来（没有的、读不懂的从空的记），以后记一份写一次。
    pub(super) fn load(file: PathBuf) -> SentFiles {
        let venues = std::fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        SentFiles(Arc::new(Held {
            file: Some(file),
            venues: Mutex::new(venues),
        }))
    }

    /// 她在场所 `venue` 传成了一份文件 `sent`：去掉过期的，满了的挤掉最早的，写盘。
    pub(super) fn note(&self, venue: &str, sent: Sent) {
        let now = sent.at;
        let bytes = {
            let mut venues = self.0.venues.lock().unwrap_or_else(PoisonError::into_inner);
            let files = venues.entry(venue.to_string()).or_default();
            files.retain(|file| now.saturating_sub(file.at) < KEEP);
            if files.len() >= PER_VENUE {
                files.pop_front();
            }
            files.push_back(sent);
            serde_json::to_vec(&*venues)
        };
        let Some(file) = &self.0.file else {
            return;
        };
        let written = bytes.map_err(|error| error.to_string()).and_then(|bytes| {
            miyu_client::places::write(file, &bytes).map_err(|error| error.to_string())
        });
        if let Err(error) = written {
            tracing::warn!(target: TARGET, file = %file.display(), error = %error, "sent files not written");
        }
    }

    /// 收到的文件 `received` 是她在场所 `venue` 不到 [`KEEP`] 以前（此刻 `now`）传过的：文件编号对得上，或者名字、字节数都对得上。
    pub(super) fn sent(&self, venue: &str, (id, name, size): Received<'_>, now: u64) -> bool {
        let venues = self.0.venues.lock().unwrap_or_else(PoisonError::into_inner);
        venues.get(venue).is_some_and(|files| {
            files.iter().any(|file| {
                now.saturating_sub(file.at) < KEEP
                    && (file.id.as_deref() == Some(id) || (file.name == name && file.size == size))
            })
        })
    }
}

/// 一条消息的段 `segments` 只带一个文件（没有字、@、引用、转发）：交回它的平台编号、名字和字节数；别的、名字或字节数读不出的是
/// 空的。
pub(super) fn receipt(segments: &Segments) -> Option<Received<'_>> {
    let only_file = segments.reply_to.is_none()
        && segments.forwards.is_empty()
        && segments.pieces.iter().all(|piece| match piece {
            Piece::Text(text) => text.trim().is_empty(),
            Piece::At(_) => false,
        });
    match segments.media.as_slice() {
        [file] if only_file && file.kind == MediaKind::File => {
            Some((file.id.as_str(), file.name.as_deref()?, file.size?))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;
