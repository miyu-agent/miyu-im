//! 引用指定的一条（施工 I-31，2026-10-23 项目主人定：以后的 loop 里她要自己决定指向谁）：`qq_quote {msg}`，私聊、群都给。`msg` 是那
//! 一条的编号（`[msg=…]`），先 `get_msg` 核对在不在这个会话里（同撤回、贴表情：人打得出别的群的编号）；在的交回「引用这一条」，im
//! 记在这条线这一轮，她的下一句话第一段引用它，这一句不再自动引用、自动 @。

use serde_json::Value;

use miyu_im::toolset::{Call, Done, Effect, Missed, Planned, message, msg_id};

use super::args::bad_args;
use super::target::recallable;

/// `qq_quote` 的一次调用（模块的说明）。参数写错的说哪个（同施工 I-17）。
pub(super) fn quote(call: Call) -> Planned {
    let Some(msg) = msg_id(&call.args["msg"]) else {
        return bad_args(&call, "msg must be a message id");
    };
    Planned::Later(Box::pin(async move { quoted(&call, msg).await }))
}

/// 核对，交回要引用的那一条和答的话。
async fn quoted(call: &Call, msg: String) -> Done {
    let Some(link) = &call.link else {
        return Done::from(refused(call, "unreachable"));
    };
    let fetched = match message(link.as_ref(), &call.to, &msg).await {
        Ok(fetched) => fetched,
        Err(Missed::Failed(_)) => return Done::from(refused(call, "not-found")),
        Err(Missed::Unanswered) => return Done::from(refused(call, "unanswered")),
        Err(_) => return Done::from(refused(call, "unreachable")),
    };
    if !recallable(&fetched, &call.to, &call.bot) {
        return Done::from(refused(call, "not-found"));
    }
    Done {
        answer: call.answers.say("quoted", &[], false),
        effect: Some(Effect::Quote(msg)),
    }
}

/// 没办成的一句（`why`，不带字段）。
fn refused(call: &Call, why: &str) -> Value {
    call.answers.say(why, &[], true)
}
