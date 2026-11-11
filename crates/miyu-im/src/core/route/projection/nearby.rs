//! 这个群里她刚说过的话（施工 I-217）：出站链的去重原来只看这一回合，两个人前后脚叫她、主线和支线各开一轮时，两轮说出一模一样
//! 的一句都发得出去（线上 2026-11-11：隔 2 秒两句「这个也不说」）。这里记她在这个群里最近过了出站链的几句（哪条线、什么时候），一条线说话时把 [`WINDOW_MILLIS`] 以内**别的线**说过的一起交给去重。同一条线前后几轮不比：那是一轮接一轮回不同的人，
//! 两个人先后问「在吗」，两句「在」都该发。只在内存里：桥重起以后是空的，不从日志补。

use std::collections::VecDeque;

use miyu_client::time::Timestamp;

use super::Projection;

/// 多久以内算「刚说过」（毫秒）：线上撞上的两回隔 2 秒、16 秒。
pub(super) const WINDOW_MILLIS: i64 = 30_000;

/// 留最近几句。
const KEPT: usize = 8;

/// 她刚说过的话，照先后。
#[derive(Debug, Default)]
pub(super) struct Nearby {
    said: VecDeque<Said>,
}

/// 她说过的一句。
#[derive(Debug)]
struct Said {
    /// 过出站链的时刻（毫秒）。
    at: i64,
    /// 哪条线。
    line: String,
    /// 过了出站链的正文。
    text: String,
}

impl Projection {
    /// 她在线 `line` 说的一句 `text` 过了出站链，此刻是 `now`：记下，别的线照它去重。
    pub(in crate::core::route) fn said_nearby(&mut self, now: Timestamp, line: &str, text: &str) {
        self.nearby.said.push_back(Said {
            at: now.unix_millis(),
            line: line.to_string(),
            text: text.to_string(),
        });
        while self.nearby.said.len() > KEPT {
            self.nearby.said.pop_front();
        }
    }

    /// 此刻 `now` 往前 [`WINDOW_MILLIS`] 以内，这个群里别的线（不是 `line`）说过的话，照先后。
    pub(in crate::core::route) fn nearby(&self, now: Timestamp, line: &str) -> Vec<String> {
        let now = now.unix_millis();
        self.nearby
            .said
            .iter()
            .filter(|said| said.line != line)
            .filter(|said| (0..=WINDOW_MILLIS).contains(&(now - said.at)))
            .map(|said| said.text.clone())
            .collect()
    }
}
