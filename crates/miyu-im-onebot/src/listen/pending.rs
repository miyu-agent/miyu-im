//! 还没升级的连接的名额（施工 I-94 起总数 `pending_connections`；施工 I-107，发布前审查 I4-1：再按来源地址分）：一个来源同时最多
//! `pending_per_source` 条，所有来源加起来最多 `pending_connections` 条。原来只有总数：连得到端口的人不带令牌就能开满总数、到时被关
//! 就再开，NapCat 断过一次就连不回来；按来源分了以后，一台机器只占得了自己那几条。
//!
//! 拿到的名额（[`Slot`]）放下就让出来：升级成了、没成、连接关了都一样。

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, PoisonError};

/// 名额：总数、每个来源的上限，和现在每个来源占着几条。
pub(super) struct Pending {
    total: usize,
    per_source: usize,
    held: Mutex<Held>,
}

/// 占着的：一共几条、每个来源几条（没占的不留键）。
#[derive(Default)]
struct Held {
    total: usize,
    sources: HashMap<IpAddr, usize>,
}

/// 拿不到名额的原因。
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Full {
    /// 这个来源的满了。
    Source,
    /// 所有来源加起来满了。
    Total,
}

impl Pending {
    /// 总数 `total`、每个来源 `per_source` 条（都至少 1，`Tuning::load` 查过）。
    pub(super) fn new(total: usize, per_source: usize) -> Arc<Pending> {
        Arc::new(Pending {
            total,
            per_source,
            held: Mutex::default(),
        })
    }

    /// 给来源 `source` 拿一个名额。
    ///
    /// # Errors
    ///
    /// 这个来源的、总数满了。
    pub(super) fn take(self: &Arc<Self>, source: IpAddr) -> Result<Slot, Full> {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        let mine = held.sources.get(&source).copied().unwrap_or_default();
        if mine >= self.per_source {
            return Err(Full::Source);
        }
        if held.total >= self.total {
            return Err(Full::Total);
        }
        held.total += 1;
        held.sources.insert(source, mine + 1);
        Ok(Slot {
            pending: Arc::clone(self),
            source,
        })
    }
}

/// 拿到的一个名额：放下就让出来。
pub(super) struct Slot {
    pending: Arc<Pending>,
    source: IpAddr,
}

impl Drop for Slot {
    fn drop(&mut self) {
        let mut held = self
            .pending
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        held.total = held.total.saturating_sub(1);
        if let Some(mine) = held.sources.get_mut(&self.source) {
            *mine = mine.saturating_sub(1);
            if *mine == 0 {
                held.sources.remove(&self.source);
            }
        }
    }
}

#[cfg(test)]
mod tests;
