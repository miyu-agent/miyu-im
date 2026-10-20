//! 起 [`Route`](super::Route) 时交进来的几样数（从 `route.rs` 挪出来，施工 O-53：那一份行数到了上限）：回执怎么撤、问到的记多久。

use std::time::Duration;

/// 群里的回执怎么撤（施工 O-25 上、O-47）：发出去几秒后撤，`im.recall_receipts` 的默认值（推来 `null` 的回到它）。
pub(crate) struct Recalling {
    /// 发出去几秒后撤（`bridge.json` 的 `receipt_recall_seconds`）。
    pub(crate) after: Duration,
    /// 撤回执的命令的默认值（清单里的）。
    pub(crate) default: Vec<String>,
}

/// 问到的记多久：「是不是终端管理员」（施工 O-31，`binding_seconds`），群名隔多久再问（施工 O-49，`group_info_seconds`）。
pub(crate) struct Asking {
    /// 「是不是终端管理员」记多久。
    pub(crate) binding: Duration,
    /// 群名隔多久再问。
    pub(crate) group_info: Duration,
    /// 禁言以后多久以内来的排着（施工 O-50，`mute_window_seconds`）。
    pub(crate) mute_window: Duration,
    /// 解禁以后最多回几条排着的（施工 O-50，`mute_replies`）。
    pub(crate) mute_replies: usize,
}

/// 一个发回话、撤回执的任务结束了：交回它交出来的（等来的结局，施工 O-25 中；从 `route.rs` 挪来，施工 T-4）。崩了（是 bug：任务里只等
/// 回应、等几秒撤）照实交上去，桥照「施工时定的」第 14 条停下，不装作还在跑；记一行运行日志。任务不会被掐掉：没有人掐它们。
pub(super) fn sent<T>(
    joined: Result<T, tokio::task::JoinError>,
) -> Result<T, crate::serve::Failure> {
    joined.map_err(|error| {
        tracing::error!(target: crate::TARGET, error = %error, "reply task crashed");
        crate::serve::Failure::Crashed(error.to_string())
    })
}
