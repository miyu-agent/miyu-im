//! 合并转发里的一条（施工 I-2 上从 `im/forwards.rs` 挪来）。

use super::Segments;

/// 合并转发里的一条。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// 发的人的号：`sender.user_id`，没有的照 `user_id`；都没有、读不出的是空的。
    pub user: Option<i64>,
    /// 发的人当时叫什么：`sender` 的群名片、昵称（同群消息的 `name`），没有的照节点自己的 `card`、`nickname`；照原样，没洗。
    pub name: Option<String>,
    /// 认出来的段，同一条消息的认法（`segments`）：`message`，没有的照 `content`。
    pub segments: Segments,
}
