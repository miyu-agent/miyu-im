//! 发消息的动作（从 `onebot.rs` 挪来，施工 O-59 下：那一份行数到了上限）：一句字、一张图、长文画成的一页、一条合并转发，
//! 照 `to` 拼成 `send_private_msg`、`send_group_msg`（合并转发是 `send_private_forward_msg`、`send_group_forward_msg`）。

use serde_json::{Value, json};

use super::{Lead, To};

/// 发一句 `text` 给 `to` 的动作和参数：`send_private_msg {user_id, message}` 或者 `send_group_msg {group_id, message}`
/// （第 10 条）。`message` 先是 `lead` 要带的引用段、@ 段，再是一个文字段（施工 O-25 上）。
pub fn message_to(to: To, text: &str, lead: &Lead) -> (&'static str, Value) {
    let mut message = lead_segments(lead);
    message.push(json!({"type": "text", "data": {"text": text}}));
    send_to(to, message)
}

/// 把一张图（`file` 写成 NapCat 认的 `base64://…`）发给 `to` 的动作和参数：一个图片段的 `send_private_msg`、`send_group_msg`
/// （施工 O-46，「展示」第 2 条）。`sticker` 的是表情包（施工 O-51）。
pub fn image_to(to: To, file: &str, sticker: bool) -> (&'static str, Value) {
    let mut data = json!({"file": file});
    if sticker {
        // 表情包（施工 O-51，设计 34 第二节第 5 条）：QQ 当表情显示。两种拼写都给（照旧版：OneBot 11 的文档写 `sub_type`，
        // go-cqhttp 系认 `subType`）。
        data["sub_type"] = json!(1);
        data["subType"] = json!(1);
    }
    send_to(to, vec![json!({"type": "image", "data": data})])
}

/// 长文画成的一页（施工 O-59 上）：先是 `lead` 要带的引用段、@ 段，再是一个图片段，同一条消息。
pub fn picture_to(to: To, file: &str, lead: &Lead) -> (&'static str, Value) {
    let mut message = lead_segments(lead);
    message.push(json!({"type": "image", "data": {"file": file}}));
    send_to(to, message)
}

/// `lead` 要带的引用段、@ 段：@ 先是 `at`，再是 `more` 里和它不重的。
fn lead_segments(lead: &Lead) -> Vec<Value> {
    let mut message = Vec::new();
    if let Some(id) = &lead.reply {
        message.push(json!({"type": "reply", "data": {"id": id}}));
    }
    let mut ats: Vec<&String> = lead.at.iter().collect();
    for qq in &lead.more {
        if !ats.contains(&qq) {
            ats.push(qq);
        }
    }
    for qq in ats {
        message.push(json!({"type": "at", "data": {"qq": qq}}));
        // @ 段和后面的字挨着画，隔一个空格好读（旧版这样发，客户端不一定自己隔开）。
        message.push(json!({"type": "text", "data": {"text": " "}}));
    }
    message
}

/// 一串段 `message` 发给 `to`：`send_private_msg {user_id, message}`、`send_group_msg {group_id, message}`。
fn send_to(to: To, message: Vec<Value>) -> (&'static str, Value) {
    match to {
        To::Private(user) => (
            "send_private_msg",
            json!({"user_id": user, "message": message}),
        ),
        To::Group(group) => (
            "send_group_msg",
            json!({"group_id": group, "message": message}),
        ),
    }
}

/// 一条合并转发（施工 O-59 下，「长文」第 2 条）：每一段 `nodes` 一个节点，发的人照机器人号 `bot`、名字 `nickname`；
/// `send_private_forward_msg {user_id, messages}`、`send_group_forward_msg {group_id, messages}`。合并转发里带不了引用、@。
pub fn forward_to(to: To, bot: i64, nickname: &str, nodes: &[String]) -> (&'static str, Value) {
    let messages: Vec<Value> = nodes
        .iter()
        .map(|text| {
            json!({"type": "node", "data": {
                "user_id": bot.to_string(),
                "nickname": nickname,
                "content": [{"type": "text", "data": {"text": text}}],
            }})
        })
        .collect();
    match to {
        To::Private(user) => (
            "send_private_forward_msg",
            json!({"user_id": user, "messages": messages}),
        ),
        To::Group(group) => (
            "send_group_forward_msg",
            json!({"group_id": group, "messages": messages}),
        ),
    }
}
