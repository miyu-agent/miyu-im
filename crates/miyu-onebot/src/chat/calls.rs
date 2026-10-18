//! 一次调用没成（施工 I-2 上从 `onebot/calls.rs` 挪来）：驱动、宿主都认这几种。

use serde_json::Value;

/// 一次调用没成。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallError {
    /// 等了驱动给的那么久，没等到回应。
    Timeout,
    /// 连接断了：发不出去，或者等的时候断了。
    Closed,
    /// 回了，`status` 不是 `ok`：交回回应原文（施工 O-25 中：出站队列取它的 `message` 记进 `failed`，`onebot.md`「施工时定的」
    /// 第 120 条）。
    Failed(Value),
}

/// 回失败时说的为什么最多留几个字符：出站队列 `failed` 的 `detail`（`onebot.md`「出站队列」第 4 条）、平台工具答的原话（施工
/// O-31）、取东西答的原话（施工 O-33）都照它。
pub const DETAIL: usize = 200;
