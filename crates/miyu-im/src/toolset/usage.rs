//! 查用量的一头（施工 I-206，[`Call::usage`](super::Call::usage)；`toolset.rs` 到了行数上限，放在这里）。

use std::path::PathBuf;

use serde_json::Value;

use super::Spender;
use crate::core::Caller;

/// 查用量的一头（施工 I-206）：im 自己照人记的账本（`crate::spent`），和经桥的连接问核心的用量账本、换算金额。
#[derive(Clone)]
pub struct Usage {
    /// im 的每人账本在哪。
    pub(crate) path: PathBuf,
    /// 跟核心的连接。
    pub(crate) caller: Caller,
}

impl Usage {
    /// 场所 `venue` 从 `from`（毫秒）起每个人用了多少，从多到少；`person` 写了的只要那一个人。
    ///
    /// # Errors
    ///
    /// 读不出库：原话。
    pub async fn spenders(
        &self,
        venue: &str,
        from: i64,
        person: Option<String>,
    ) -> Result<Vec<Spender>, String> {
        let (path, venue) = (self.path.clone(), venue.to_string());
        tokio::task::spawn_blocking(move || {
            crate::spent::spenders(&path, &venue, from, person.as_deref())
        })
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
    }

    /// 调核心的 `method`（`usage.query`、`usage.convert`），交回 `result`；核心拒了、断开了的是空的。
    pub async fn core(&self, method: &str, params: Value) -> Option<Value> {
        let reply = self.caller.call(method, params).await.ok()?;
        crate::core::reason(&reply)
            .is_none()
            .then(|| reply["result"].clone())
    }
}
