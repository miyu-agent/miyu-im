//! 撤回、贴表情（施工 I-185 下上，设计 07 第 3.5 节「说话那一块：贴表情、撤回自己的」；照 QQ 的 `qq_recall`、`qq_react`）：
//!
//! - `tg_recall {msg?}`：撤她给的编号那一条；没给的撤叫她做的那条引用的那一条，都没有的答 `no-quote`。Telegram 上机器人 48 小时内
//!   撤得了自己的，是群管理员时也撤得了别人的；撤不了的照 Telegram 的原话答。
//! - `tg_react {msg, emoji}`：在那一条上贴一个表情。Telegram 只认它给的那一套表情，一条消息机器人只能贴一个；不认的照原话答。
//! - 她给的编号先照 im 的聊天记录核对在这个场所里（Telegram 的编号是一个聊天里自己数的，别处的同一个号对不上；它也不给机器人
//!   照编号取消息），没有的答 `not-found`。

use serde_json::Value;

use miyu_im::chat::CallError;
use miyu_im::toolset::common::args::bad_args;
use miyu_im::toolset::{Call, Planned, msg_id};

/// `tg_recall` 的一次调用（模块的说明）。
pub(super) fn recall(call: Call) -> Planned {
    let given = match &call.args["msg"] {
        Value::Null => None,
        value => match msg_id(value) {
            Some(msg) => Some(msg),
            None => return bad_args(&call, "msg must be a message id"),
        },
    };
    let quoted = call
        .origin
        .as_ref()
        .and_then(|origin| origin.quote.as_ref())
        .map(|quote| quote.msg.clone());
    let (msg, check) = match (given, quoted) {
        (Some(msg), _) => (msg, true),
        (None, Some(msg)) => (msg, false),
        (None, None) => return Planned::Now(call.answers.say("no-quote", &[], true)),
    };
    Planned::later(async move {
        if check && let Some(refused) = unseen(&call, &msg).await {
            return refused;
        }
        let Some(link) = &call.link else {
            return call.answers.say("unreachable", &[], true);
        };
        match link.recall(&call.to, &msg).await {
            Ok(()) => call.answers.say("recalled", &[], false),
            Err(error) => undone(&call, error),
        }
    })
}

/// `tg_react` 的一次调用（模块的说明）。
pub(super) fn react(call: Call) -> Planned {
    let Some(msg) = msg_id(&call.args["msg"]) else {
        return bad_args(&call, "msg must be a message id");
    };
    let Some(emoji) = call.args["emoji"]
        .as_str()
        .map(str::trim)
        .filter(|emoji| !emoji.is_empty())
        .map(str::to_string)
    else {
        return bad_args(&call, "emoji must be one emoji");
    };
    Planned::later(async move {
        if let Some(refused) = unseen(&call, &msg).await {
            return refused;
        }
        let Some(link) = &call.link else {
            return call.answers.say("unreachable", &[], true);
        };
        match link.react(&call.to, &msg, &emoji, true).await {
            Ok(()) => call.answers.say("reacted", &[], false),
            Err(error) => undone(&call, error),
        }
    })
}

/// 编号是 `msg` 的那一条不在 im 记下的这个场所里：答的那一句；在的是空的。
async fn unseen(call: &Call, msg: &str) -> Option<Value> {
    match call.history.sender_of(&call.venue, msg).await {
        Ok(Some(_)) => None,
        Ok(None) => Some(call.answers.say("not-found", &[], true)),
        Err(detail) => Some(call.answers.say("failed", &[("detail", &detail)], true)),
    }
}

/// 没办成的那一句：Telegram 回了没成的照原话，等不到的说可能办了，别的说够不着。
fn undone(call: &Call, error: CallError) -> Value {
    match error {
        CallError::Failed { said, .. } => call.answers.say("failed", &[("detail", &said)], true),
        CallError::Timeout => call.answers.say("unanswered", &[], true),
        _ => call.answers.say("unreachable", &[], true),
    }
}
