//! 一次调用没成（施工 I-2 上从 `onebot/calls.rs` 挪来）：驱动、宿主都认这几种。

/// 一次调用没成。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallError {
    /// 等了驱动给的那么久，没等到回应。
    Timeout,
    /// 连接断了：发不出去，或者等的时候断了。
    Closed,
    /// 平台回了没成（施工 I-2 上：驱动读好了交过来，宿主不看平台的回应原文）。
    Failed {
        /// 平台说的为什么，截到 [`DETAIL`] 个字符：平台工具、取东西答的原话照它（施工 O-31、O-33）。
        said: String,
        /// 平台自己那一句话，去掉首尾空白，没有的是空的：出站队列的 `failed` 照它记 `detail`（施工 O-25 中，`onebot.md`
        /// 「施工时定的」第 120 条）。
        message: String,
    },
}

/// 回失败时说的为什么最多留几个字符：出站队列 `failed` 的 `detail`（`onebot.md`「出站队列」第 4 条）、平台工具答的原话（施工
/// O-31）、取东西答的原话（施工 O-33）都照它。
pub const DETAIL: usize = 200;
