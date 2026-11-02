//! 令牌猜错的限次（施工 I-117，发布前审查 I4-3：NapCat 的端口出厂听 `0.0.0.0`，猜令牌原来只受网速限制，每次猜错记一行 WARN、日志也能
//! 被刷大）：照核心密码登录的限流，一个来源地址 [`WINDOW`] 以内猜错 [`TRIES`] 次，这一段剩下的时候它连进来一律 429、不再比令牌。
//!
//! 运行日志：一段里头一次猜错记一行（带来源），挡住的那一下再记一行带次数；中间的不记。

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

/// 一段多长。
pub(super) const WINDOW: Duration = Duration::from_secs(60);

/// 一段里最多猜错几次。
pub(super) const TRIES: u32 = 5;

/// 每个来源这一段的：哪一刻开始的、猜错了几次。
#[derive(Default)]
pub(super) struct Guesses {
    sources: Mutex<HashMap<IpAddr, (Instant, u32)>>,
}

/// 猜错一次以后怎么记：头一次、挡住了、都不是。
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Noted {
    /// 这一段头一次猜错：记一行。
    First,
    /// 这一次够了 [`TRIES`]：记一行，往后这一段挡住。
    Blocked(u32),
    /// 中间的：不记。
    Quiet,
}

impl Guesses {
    /// 来源 `source` 这时挡没挡住（这一段猜错够了）。
    pub(super) fn blocked(&self, source: IpAddr, now: Instant) -> bool {
        let sources = self.sources.lock().unwrap_or_else(PoisonError::into_inner);
        sources
            .get(&source)
            .is_some_and(|(since, tries)| now.duration_since(*since) < WINDOW && *tries >= TRIES)
    }

    /// 来源 `source` 猜错了一次。过了一段的从头数；顺手扔掉过了一段的别的来源，表不长。
    pub(super) fn missed(&self, source: IpAddr, now: Instant) -> Noted {
        let mut sources = self.sources.lock().unwrap_or_else(PoisonError::into_inner);
        sources.retain(|_, (since, _)| now.duration_since(*since) < WINDOW);
        let (_, tries) = sources.entry(source).or_insert((now, 0));
        *tries += 1;
        match *tries {
            1 => Noted::First,
            count if count == TRIES => Noted::Blocked(count),
            _ => Noted::Quiet,
        }
    }
}

#[cfg(test)]
mod tests;
