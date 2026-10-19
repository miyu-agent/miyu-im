//! 一条要交给核心的消息（从 `route.rs` 挪出来，施工 O-60：那一份行数到了上限）；她一条回话里的文字（施工 I-7 上补挪过来，同一个
//! 原因）。

use serde_json::Value;

use super::forwards::Forwards;
use super::pictures::Pictures;
use super::session::Place;

/// 一条要交给核心的消息（私聊的、群的）。
pub(super) struct Message {
    /// 命令编号（第 8 条）。
    pub(super) id: String,
    /// 平台的消息编号：只记运行日志。
    pub(super) number: i64,
    /// 交给核心的正文。
    pub(super) text: String,
    /// `as`：发的人的平台身份，群里另带 `role`（「群消息」第 4 条）。
    pub(super) acting: Value,
    /// `session.send` 的 `venue`（施工 O-22，`fields`）。
    pub(super) fields: Value,
    /// 它的场所：怎么找会话、回执发到哪。
    pub(super) place: Place,
    /// 交以前要取的图（施工 O-33，`pictures`）：群里冲她来的、私聊收下的才有。
    pub(super) pictures: Option<Pictures>,
    /// 交以前要展开的合并转发（施工 O-34，`forwards`）：没有转发的是空的。
    pub(super) forwards: Option<Forwards>,
}

/// 她一条回话里的文字：`text` 块依次接起来，思考、工具调用不要（第 10 条）。
pub(super) fn reply_text(event: &Value) -> String {
    event["body"]["blocks"]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter(|block| block["type"] == "text")
                .filter_map(|block| block["text"].as_str())
                .collect()
        })
        .unwrap_or_default()
}
