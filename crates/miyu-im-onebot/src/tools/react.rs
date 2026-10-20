//! 给消息贴表情（施工 I-12，设计 18 第十一节「互动」）：`qq_react {msg, emoji}`，只给群。`msg` 是那一条的编号（`[msg=…]`），先 `get_msg`
//! 核对在不在这个会话里（同撤回她给的编号，人打得出别的群的编号）；`emoji` 是一个 Unicode 表情（`👍`），照 QQ 的写法换成码点的十进制，
//! 写成数字的当 QQ 的系统表情编号。QQ 不认的照它回的原话答。

use serde_json::Value;

use miyu_im::chat::To;
use miyu_im::toolset::{Call, Missed, Planned, message, msg_id};

use super::lookup::undone;
use super::target::recallable;

/// 读 `emoji`（纯逻辑）：全是数字的照原样（系统表情编号）；不然头一个字的码点的十进制。空的是空的。
pub(crate) fn emoji_id(emoji: &str) -> Option<String> {
    let emoji = emoji.trim();
    if emoji.is_empty() {
        return None;
    }
    if emoji.bytes().all(|byte| byte.is_ascii_digit()) {
        return Some(emoji.to_string());
    }
    emoji
        .chars()
        .next()
        .map(|first| u32::from(first).to_string())
}

/// `qq_react` 的一次调用（模块的说明）。参数写错的、私聊里调到的照不认识的工具答。
pub(super) fn react(call: Call) -> Planned {
    let wanted = msg_id(&call.args["msg"]).zip(call.args["emoji"].as_str().and_then(emoji_id));
    let (To::Group(_), Some((msg, emoji))) = (call.to, wanted) else {
        return Planned::Now(call.answers.say("unknown", &[("name", &call.tool)], true));
    };
    Planned::later(async move { reacted(&call, &msg, &emoji).await })
}

/// 核对、贴，答那一句。
async fn reacted(call: &Call, msg: &str, emoji: &str) -> Value {
    let Some(link) = &call.link else {
        return call.answers.say("unreachable", &[], true);
    };
    let fetched = match message(link.as_ref(), msg).await {
        Ok(fetched) => fetched,
        Err(Missed::Failed(_)) => return call.answers.say("not-found", &[], true),
        Err(Missed::Unanswered) => return call.answers.say("unanswered", &[], true),
        Err(_) => return call.answers.say("unreachable", &[], true),
    };
    if !recallable(&fetched, call.to, call.bot) {
        return call.answers.say("not-found", &[], true);
    }
    match link.react(msg, emoji, true).await {
        Ok(()) => call.answers.say("reacted", &[], false),
        Err(error) => {
            let (why, detail) = undone(error);
            let fields: Vec<(&str, &str)> = detail
                .as_deref()
                .map(|detail| ("detail", detail))
                .into_iter()
                .collect();
            call.answers.say(why, &fields, true)
        }
    }
}

#[cfg(test)]
mod tests;
