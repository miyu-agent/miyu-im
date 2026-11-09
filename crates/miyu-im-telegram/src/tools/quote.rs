//! 引用指定的一条（同 QQ 的 `qq_quote`，施工 I-31）：`tg_quote {msg}`，私聊、群都给。Telegram 没有「照编号取一条消息」，照 im 的聊天
//! 记录核对这一条在不在这个场所里（编号是一个聊天里自己数的，别处的同一个号对不上），在的交回「引用这一条」：im 记在这条线这一轮，
//! 她的下一句话第一段引用它。

use miyu_im::toolset::common::args::bad_args;
use miyu_im::toolset::{Call, Done, Effect, Planned, msg_id};

/// 一次调用（模块的说明）。
pub(super) fn quote(call: Call) -> Planned {
    let Some(msg) = msg_id(&call.args["msg"]) else {
        return bad_args(&call, "msg must be a message id");
    };
    Planned::Later(Box::pin(async move {
        match call.history.sender_of(&call.venue, &msg).await {
            Ok(Some(_)) => Done {
                answer: call.answers.say("quoted", &[], false),
                effect: Some(Effect::Quote(msg)),
            },
            Ok(None) => Done::from(call.answers.say("not-found", &[], true)),
            Err(detail) => Done::from(call.answers.say("failed", &[("detail", &detail)], true)),
        }
    }))
}
