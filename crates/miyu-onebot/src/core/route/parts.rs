//! 起 [`Route`](super::Route) 时交进来的几样数（从 `route.rs` 挪出来，施工 O-53：那一份行数到了上限）：回执怎么撤、问到的记多久。

use std::time::Duration;

/// 群里的回执怎么撤（施工 O-25 上、O-47）：发出去几秒后撤，`onebot.recall_receipts` 的默认值（推来 `null` 的回到它）。
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
