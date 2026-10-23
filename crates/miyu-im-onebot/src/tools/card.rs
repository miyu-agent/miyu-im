//! 改群名片（施工 I-36，2026-10-23 项目主人定：她自己的、别人的都能改，「让它自己判断就行，这个没什么危害不用做限制」）：
//! `qq_set_card {user?, card}`，只给群。没写 `user` 的改她自己的；写了的改那个人的，成不成由 QQ 说（她不是管理员、改的是群主和
//! 管理员的会被拒，照 QQ 的原话答）。`card` 去掉首尾空白，空的是清掉群名片（显示回昵称），最多 [`MOST`] 个字。

use serde_json::{Value, json};

use miyu_im::chat::To;
use miyu_im::toolset::{Call, Planned};

use super::args::{bad_args, group_only, qq_number};
use super::lookup::{said, undone};

/// 群名片最多几个字（QQ 的上限照字节算，60 个字够用；多了的报出来让她改短）。
const MOST: usize = 60;

/// 读 `qq_set_card` 的参数（纯逻辑）：（改谁，空的是她自己；改成什么）。写错的交回哪个参数不对。
pub(crate) fn card(args: &Value) -> Result<(Option<i64>, String), &'static str> {
    let user = match &args["user"] {
        Value::Null => None,
        user => Some(qq_number(user).ok_or("user must be a QQ number")?),
    };
    let card = args["card"]
        .as_str()
        .ok_or("card must be text")?
        .trim()
        .to_string();
    if card.chars().count() > MOST {
        return Err("card must be at most 60 characters");
    }
    Ok((user, card))
}

/// `qq_set_card` 的一次调用（模块的说明）。
pub(super) fn set_card(call: Call) -> Planned {
    let To::Group(group) = call.to else {
        return group_only(&call);
    };
    let (user, card) = match card(&call.args) {
        Ok(read) => read,
        Err(bad) => return bad_args(&call, bad),
    };
    Planned::later(async move { renamed(&call, group, user, &card).await })
}

/// 改，答那一句。
async fn renamed(call: &Call, group: i64, user: Option<i64>, card: &str) -> Value {
    let Some(link) = &call.link else {
        return call.answers.say("unreachable", &[], true);
    };
    let target = user.unwrap_or(call.bot);
    let params = json!({"group_id": group, "user_id": target, "card": card});
    let reply = match link.raw("set_group_card", params).await {
        Ok(reply) if reply["status"] == "ok" => reply,
        Ok(reply) => return said(call, undone(crate::onebot::failed(&reply))),
        Err(error) => return said(call, undone(error)),
    };
    drop(reply);
    match user.filter(|user| *user != call.bot) {
        None => call.answers.say("renamed-me", &[("card", card)], false),
        Some(user) => {
            let who = call
                .names
                .get(&user)
                .cloned()
                .unwrap_or_else(|| user.to_string());
            call.answers
                .say("renamed", &[("who", &who), ("card", card)], false)
        }
    }
}

#[cfg(test)]
mod tests;
