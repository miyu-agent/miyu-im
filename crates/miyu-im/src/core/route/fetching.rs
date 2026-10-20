//! 取东西的几个数（`bridge.json`，施工 O-33）：冲她来的那条带的图交消息以前取（`pictures`）。按需取（`fetch_media`）施工 T-3 挪进
//! 驱动（`miyu-im-onebot/src/tools/fetch.rs`），它的几个数跟着走了。

use std::time::Duration;

/// 取东西的几个数。
#[derive(Debug, Clone)]
pub(crate) struct Fetching {
    /// 冲她来的那条（和它引用的那条）各最多带几张图（`message_images`）。
    pub(crate) images: usize,
    /// 冲她来的那条的图等多久（`call_timeout_seconds`）：交消息以前取，不能等太久。
    pub(crate) quick: Duration,
}
