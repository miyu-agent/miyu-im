//! @ 指定的人（施工 O-55，`onebot.md` 第一条「@ 指定的人」，2026-10-17 项目主人定，照旧版的 `qq_mention_users`；施工 T-4 从宿主挪进
//! 驱动）：群里她调 `mention {users}`，照号认是不是这个群的成员：先看 im 交来的这个群缓存着的名字，有没缓存的就取一次群成员表
//! （`get_group_member_list`，施工 I-17 照旧版：以前一个个问，一个问不到后面的全当不是成员）。都是的交回「@ 这几个号」（im 记在这条线
//! 这一轮，她的下一句话第一段前面 @ 他们）；有不是的整次不记，答哪几个不是（同旧版）。号写错的答写错了（施工 I-17），私聊里调到的
//! 答只给群。

use std::collections::HashMap;

use serde_json::Value;

use miyu_im::chat::To;
use miyu_im::toolset::{Call, Done, Effect, NAME, Planned, clean};

use super::args::{bad_args, group_only, qq_numbers};
use super::lookup::{said, undone};

/// 一次最多 @ 几个人（施工 I-17，同旧版 `MAX_TOOL_MENTIONS`）。
const MOST: usize = 32;

/// 读 `mention` 的参数 `args` 里的号（纯逻辑）：`users` 是正整数（写成字的也认）的列表，去重、照先后，1 到 [`MOST`] 个；别的是空的。
pub(crate) fn users(args: &Value) -> Option<Vec<i64>> {
    qq_numbers(&args["users"], MOST)
}

/// `mention` 的一次调用（模块的说明）。
pub(super) fn mention(call: Call) -> Planned {
    let To::Group(group) = call.to else {
        return group_only(&call);
    };
    let Some(users) = users(&call.args) else {
        return bad_args(&call, "users must be 1 to 32 QQ numbers");
    };
    Planned::Later(Box::pin(async move {
        let listed = match listed(&call, group, &users).await {
            Ok(listed) => listed,
            Err(error) => return Done::from(said(&call, undone(error))),
        };
        let (names, strangers) = named(&users, &call.names, &listed);
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

/// 这个群的成员表（号 → 名字，没洗）：`users` 都在缓存里的不用取，是空的。
async fn listed(
    call: &Call,
    group: i64,
    users: &[i64],
) -> Result<HashMap<i64, Option<String>>, miyu_im::chat::CallError> {
    if users.iter().all(|user| call.names.contains_key(user)) {
        return Ok(HashMap::new());
    }
    let Some(link) = &call.link else {
        return Err(miyu_im::chat::CallError::Closed);
    };
    let members = link.members(group).await?;
    Ok(members
        .into_iter()
        .map(|member| (member.user, member.name))
        .collect())
}

/// 照缓存着的名字 `cached`、成员表 `listed` 认 `users`（纯逻辑）：交回（成员的名字，照先后；不是成员的号）。名字洗过，洗完空了的
/// 写号。
pub(crate) fn named(
    users: &[i64],
    cached: &HashMap<i64, String>,
    listed: &HashMap<i64, Option<String>>,
) -> (Vec<String>, Vec<String>) {
    let mut names = Vec::new();
    let mut strangers = Vec::new();
    for user in users {
        let name = match (cached.get(user), listed.get(user)) {
            (Some(name), _) => Some(Some(name.clone())),
            (None, Some(name)) => Some(name.as_deref().and_then(|name| clean(name, NAME))),
            (None, None) => None,
        };
        match name {
            Some(name) => names.push(name.unwrap_or_else(|| user.to_string())),
            None => strangers.push(user.to_string()),
        }
    }
    (names, strangers)
}

#[cfg(test)]
mod tests;
