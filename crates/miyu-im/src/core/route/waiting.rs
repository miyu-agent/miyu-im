//! 支线都占满时排着的（施工 I-16，照旧版：一人一轮，排队有上限）：群聊内核分派成 `Queue` 的（主线在回别人、支线满了），不再
//! 并进主线这一轮（O-36 那样一轮里回好几个人容易串话），照先后排在这个群会话后面；主线这一轮结束了（`turn.ended`）开下一轮。
//!
//! - 每个群会话最多排 [`MOST`] 轮：满了的新来的只记下（判断里照旧是要回），记一行 `WARN reply queue full`。
//! - 只在内存里：桥重启了排着的丢了（同出站队列排着的）。

use std::collections::{HashMap, VecDeque};

use super::Route;
use super::judges::Tag;
use crate::TARGET;
use crate::core::Gone;

/// 一个群会话最多排几轮（旧版 `session_queue_limit`）。
pub(super) const MOST: usize = 16;

/// 排着的：群会话 → 先后排着的几轮（判下来要回的几条的序号、最后一条的标记）。
#[derive(Debug, Default)]
pub(super) struct Waiting(HashMap<String, VecDeque<(Vec<u64>, Tag)>>);

impl Waiting {
    /// 排一轮：满了的不排，交回假。
    pub(super) fn push(&mut self, session: &str, msgs: &[u64], tag: &Tag) -> bool {
        let queue = self.0.entry(session.to_string()).or_default();
        if queue.len() >= MOST {
            return false;
        }
        queue.push_back((msgs.to_vec(), tag.clone()));
        true
    }

    /// 去掉含序号 `seq` 的几轮（施工 I-16：那一条被撤回了），交回去掉了几轮。
    pub(super) fn drop_msg(&mut self, session: &str, seq: u64) -> usize {
        let Some(queue) = self.0.get_mut(session) else {
            return 0;
        };
        let before = queue.len();
        queue.retain(|(msgs, _)| !msgs.contains(&seq));
        before - queue.len()
    }

    /// 取最前面的一轮。
    pub(super) fn pop(&mut self, session: &str) -> Option<(Vec<u64>, Tag)> {
        let queue = self.0.get_mut(session)?;
        let next = queue.pop_front();
        if queue.is_empty() {
            self.0.remove(session);
        }
        next
    }
}

impl Route {
    /// 群会话 `session` 的主线这一轮结束了：排着的头一轮照分派开（`lanes`）。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn next_waiting(&mut self, session: &str) -> Result<(), Gone> {
        if let Some((msgs, tag)) = self.queued.pop(session) {
            tracing::info!(target: TARGET, venue = %tag.venue, message = tag.number, "queued reply started");
            // 开下一轮可能又收到一轮结束（补收推送时），装箱断开递归。
            Box::pin(self.dispatched(session, &msgs, &tag)).await?;
        }
        Ok(())
    }

    /// 支线都占满了：排上。交回排上了没有。
    pub(super) fn wait(&mut self, session: &str, msgs: &[u64], tag: &Tag) -> bool {
        let queued = self.queued.push(session, msgs, tag);
        if queued {
            tracing::info!(target: TARGET, venue = %tag.venue, message = tag.number, "reply queued");
        } else {
            tracing::warn!(target: TARGET, venue = %tag.venue, message = tag.number, most = MOST, "reply queue full");
        }
        queued
    }
}

#[cfg(test)]
mod tests;
