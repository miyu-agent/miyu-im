//! @ 指定的人（施工 O-55，`onebot.md` 第一条「@ 指定的人」，2026-10-17 项目主人定，照旧版的 `qq_mention_users`；施工 T-4 从宿主挪进
//! 驱动）：群里她调 `mention {users}`，照号一个个认是不是这个群的成员（先看 im 交来的这个群缓存着的名字，没有的问 QQ 的群成员信息，
//! 问不到一次以后的不再问），都是的交回「@ 这几个号」（im 记在这条线这一轮，她的下一句话第一段前面 @ 他们）；有不是的整次不记，答哪几
//! 个不是；写错的、群外调的答 `no-target`。

use serde_json::Value;

use miyu_im::chat::To;
use miyu_im::toolset::{Call, Done, Effect, NAME, Planned, clean};

/// 一次最多 @ 几个人（一段话前面挂太多 @ 就是刷屏）。
const MOST: usize = 10;

/// 读 `mention` 的参数 `args` 里的号（纯逻辑）：`users` 是正整数（写成字的也认）的列表，去重、照先后，1 到 [`MOST`] 个；别的是空的。
pub(crate) fn users(args: &Value) -> Option<Vec<i64>> {
    let list = args["users"].as_array()?;
    let mut found: Vec<i64> = Vec::new();
    for one in list {
        let number = match one {
            Value::Number(number) => number.as_i64(),
            Value::String(text) => text.trim().parse().ok(),
            _ => None,
        }
        .filter(|number| *number > 0)?;
        if !found.contains(&number) {
            found.push(number);
        }
    }
    (1..=MOST).contains(&found.len()).then_some(found)
}

/// `mention` 的一次调用（模块的说明）。
pub(super) fn mention(call: Call) -> Planned {
    let (To::Group(group), Some(users)) = (call.to, users(&call.args)) else {
        return Planned::Now(call.answers.say("no-target", &[], true));
    };
    Planned::Later(Box::pin(async move {
        let mut names = Vec::new();
        let mut strangers = Vec::new();
        let mut asking = true;
        for user in &users {
            match name(&call, group, *user, &mut asking).await {
                Some(name) => names.push(name),
                None => strangers.push(user.to_string()),
            }
        }
        if !strangers.is_empty() {
            let who = strangers.join(", ");
            return Done::from(call.answers.say("not-members", &[("who", &who)], true));
        }
        let who = names.join(", ");
        Done {
            answer: call.answers.say("mentioned", &[("who", &who)], false),
            effect: Some(Effect::Mention(users)),
        }
    }))
}

/// 群 `group` 里号是 `user` 的人叫什么：先看缓存着的，没有的问 QQ（`asking` 是假的不问；问不到的放成假，后面的不再问）。不是
/// 这个群的成员、问不到的是空的。
async fn name(call: &Call, group: i64, user: i64, asking: &mut bool) -> Option<String> {
    if let Some(name) = call.names.get(&user) {
        return Some(name.clone());
    }
    if !*asking {
        return None;
    }
    let asked = match &call.link {
        Some(link) => link.member(group, user).await.ok(),
        None => None,
    };
    let Some(name) = asked.and_then(|member| member.name) else {
        *asking = false;
        return None;
    };
    clean(&name, NAME)
}

#[cfg(test)]
mod tests;
