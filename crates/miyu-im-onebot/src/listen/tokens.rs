//! 每个听着的端口现在的令牌（施工 I-81 下一个端口一个；施工 AUD-2，审计 #104：换了令牌断开已经连着的）：
//!
//! - 连进来的那一下照这个端口最新的比（`super::admitted`）。
//! - 连接表换上来（`accepting` 的 `publish`）一并告诉连着的各条：自己那个端口还在表里、令牌却不是当初连进来出示的那一个了（换了、
//!   删了），就断开（[`Tokens::revoked`]），NapCat 用新的令牌重连。端口不在表里了的不算换令牌，已经连着的照旧（关端口不断连接）。

use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use tokio::sync::watch;

use miyu_client::settings::Secret;

/// 端口 → 令牌（没设的是空的），换了就通知。
pub(super) struct Tokens {
    current: Mutex<BTreeMap<u16, Option<Secret>>>,
    changed: watch::Sender<()>,
}

impl Default for Tokens {
    fn default() -> Self {
        Tokens {
            current: Mutex::default(),
            changed: watch::channel(()).0,
        }
    }
}

impl Tokens {
    /// 换成 `current` 这一份，告诉连着的各条去看自己的。
    pub(super) fn set(&self, current: BTreeMap<u16, Option<Secret>>) {
        *self.current.lock().unwrap_or_else(PoisonError::into_inner) = current;
        self.changed.send_replace(());
    }

    /// 端口 `port` 现在的令牌：没设的、不在表里的是空的。
    pub(super) fn of(&self, port: u16) -> Option<Secret> {
        self.current
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&port)
            .cloned()
            .flatten()
    }

    /// 端口 `port` 还在表里、令牌却不再是 `admitted`（换了、删了）的那一刻交回；一直没换的一直等着。
    pub(super) async fn revoked(&self, port: u16, admitted: &str) {
        // 先订、后看：看完到等之间换的也收得到。
        let mut changed = self.changed.subscribe();
        loop {
            if self.no_longer(port, admitted) {
                return;
            }
            if changed.changed().await.is_err() {
                // 发的一头不在了（桥在停）：不会再换。
                std::future::pending::<()>().await;
            }
        }
    }

    /// `port` 在表里、令牌不是 `admitted`。
    fn no_longer(&self, port: u16, admitted: &str) -> bool {
        let current = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        match current.get(&port) {
            None => false,
            Some(token) => !token
                .as_ref()
                .is_some_and(|token| super::same(admitted.as_bytes(), token.expose().as_bytes())),
        }
    }
}

#[cfg(test)]
mod tests;
