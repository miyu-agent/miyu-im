//! `chat.media` 分段交（施工 I-154）：核心收一行最多 1 MiB，超了直接断开连接。原来整张图写成 base64 回在一行里，大于约 750 KB 的
//! 图一回，核心就把 im 断开、im 整个退出重来（11-04 12:34 主人的部署上出过一次：她存一张群里的大图，所有群断了一下，图也没存成）。
//!
//! - 带 `offset` 的：从那里交 [`PAGE`] 字节（写成 base64 约 683 KiB，一行放得下），带整张多大（`size`）、交完没有（`eof`）；`offset`
//!   超出整张的拒 `bad_params`。
//! - 不带的（表情包 M-12 以前）：整张在 [`PAGE`] 以内的照旧整张交，大的拒 `too_large`，不再撑断连接。
//! - 取到的整张留在 [`Recent`] 里一会儿：分段要的每一段都是一次 `chat.media`，不留的话每段都要重新问 QQ、重新下。

use std::collections::VecDeque;
use std::ops::Range;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde_json::Value;

/// 一段最多多少字节。
pub(crate) const PAGE: usize = 512 * 1024;

/// 取到的整张留多久。
const KEEP: Duration = Duration::from_secs(120);

/// 最多留几张。
const HELD: usize = 4;

/// 这一次交哪一段、交完没有（纯逻辑）：`offset` 不写的整张在 [`PAGE`] 以内才交，大的是 `too_large`；写了的超出整张是 `bad_params`。
pub(crate) fn page(size: usize, offset: Option<u64>) -> Result<(Range<usize>, bool), &'static str> {
    let Some(offset) = offset else {
        return if size <= PAGE {
            Ok((0..size, true))
        } else {
            Err("too_large")
        };
    };
    let from = usize::try_from(offset)
        .ok()
        .filter(|from| *from <= size)
        .ok_or("bad_params")?;
    let to = from.saturating_add(PAGE).min(size);
    Ok((from..to, to == size))
}

/// 取到的一整张：字节和答的别的格（种类、文件名、来历）。
pub(crate) struct Taken {
    pub(crate) bytes: Vec<u8>,
    pub(crate) answer: Value,
}

/// 留着的一张：认它的键、什么时候取到的、整张。
type Held = (String, Instant, Arc<Taken>);

/// 取到的整张，照（场所、消息编号、第几样）认。
#[derive(Clone, Default)]
pub(crate) struct Recent(Arc<Mutex<VecDeque<Held>>>);

impl std::fmt::Debug for Recent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Recent")
    }
}

impl Recent {
    /// 留着、没过期的那一张。
    pub(crate) fn get(&self, key: &str) -> Option<Arc<Taken>> {
        let mut held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        held.retain(|(_, at, _)| at.elapsed() < KEEP);
        held.iter()
            .find(|(one, _, _)| one == key)
            .map(|(_, _, taken)| Arc::clone(taken))
    }

    /// 留下一张，多了扔最早的。
    pub(crate) fn put(&self, key: String, taken: Arc<Taken>) {
        let mut held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        held.retain(|(one, _, _)| *one != key);
        held.push_back((key, Instant::now(), taken));
        while held.len() > HELD {
            held.pop_front();
        }
    }
}

#[cfg(test)]
mod tests;
