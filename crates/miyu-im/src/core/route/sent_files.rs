//! 她发出去的文件（施工 I-247，2026-11-17 项目主人验收第 35 条）：私聊里对方下载了她传的文件，QQ 那边出一条灰色的「你已成功
//! 下载文件」，NapCat 推来的却是一条普通的私聊消息：发的人是对方，只有一个文件段，就是她刚传的那个。原来照对方发了一个文件
//! 开一轮，她就回了一句。
//!
//! - 她传成了的文件（`qq_send_file`、`show` 传成文件的）按场所记下名字和字节数，只在内存里：每个场所最近 [`PER_VENUE`] 份，
//!   [`KEEP`] 以后作废；桥重启就没了（重启以前传的被下载，照旧开一轮，下载多半就在传完几分钟里）。
//! - 一条消息没有字、没有 @、没有引用和转发，只带一个文件，名字、字节数都对得上这个场所记着的一份：是下载的回执（[`receipt`]）。

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::chat::{MediaKind, Piece, Segments};

/// 记多久。
const KEEP: Duration = Duration::from_secs(24 * 60 * 60);

/// 每个场所最多记几份。
const PER_VENUE: usize = 16;

/// 场所 → 她传成了的文件（名字、字节数、什么时候），照先后。跟核心的那一头和传文件的任务共用一份。
#[derive(Debug, Clone, Default)]
pub(super) struct SentFiles(Arc<Mutex<HashMap<String, VecDeque<Sent>>>>);

/// 传成了的一份：名字、字节数、什么时候。
type Sent = (String, u64, Instant);

impl SentFiles {
    /// 她在场所 `venue` 传成了一份文件，名字 `name`、`size` 个字节，此刻 `now`。
    pub(super) fn note(&self, venue: &str, (name, size): (&str, u64), now: Instant) {
        let mut held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let files = held.entry(venue.to_string()).or_default();
        files.retain(|(_, _, at)| now.duration_since(*at) < KEEP);
        if files.len() == PER_VENUE {
            files.pop_front();
        }
        files.push_back((name.to_string(), size, now));
    }

    /// 她在场所 `venue` 不到 [`KEEP`] 以前传过名字 `name`、`size` 个字节的文件。
    pub(super) fn sent(&self, venue: &str, (name, size): (&str, u64), now: Instant) -> bool {
        let held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        held.get(venue).is_some_and(|files| {
            files.iter().any(|(sent, bytes, at)| {
                sent == name && *bytes == size && now.duration_since(*at) < KEEP
            })
        })
    }
}

/// 一条消息的段 `segments` 只带一个文件（没有字、@、引用、转发）：交回它的名字和字节数；别的、名字或字节数读不出的是空的。
pub(super) fn receipt(segments: &Segments) -> Option<(&str, u64)> {
    let only_file = segments.reply_to.is_none()
        && segments.forwards.is_empty()
        && segments.pieces.iter().all(|piece| match piece {
            Piece::Text(text) => text.trim().is_empty(),
            Piece::At(_) => false,
        });
    match segments.media.as_slice() {
        [file] if only_file && file.kind == MediaKind::File => {
            Some((file.name.as_deref()?, file.size?))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;
