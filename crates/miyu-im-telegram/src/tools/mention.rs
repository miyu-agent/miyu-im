//! @ 指定的人（同 QQ 的 `qq_mention`，施工 O-55、I-17）：`tg_mention {users}`，只给群。Telegram 不给机器人列群成员，认人照 im 交来的这个
//! 群缓存着的名字，没缓存的照 im 的聊天记录：在这个群里说过话的算成员（设计 07 第 3.5 节「识人术：只认得说过话的人」）。都认得的交回
//! 「@ 这几个号」；有不认得的整次不记，答哪几个不认得。

use std::collections::HashMap;

use miyu_im::chat::{Id, To};
use miyu_im::toolset::common::args::{bad_args, group_only};
use miyu_im::toolset::common::mention::{named, users};
use miyu_im::toolset::{Call, Done, Effect, HistoryQuery, Planned};

/// 一次调用（模块的说明）。
pub(super) fn mention(call: Call) -> Planned {
    if !matches!(call.to, To::Group(_)) {
        return group_only(&call);
    }
    let Some(users) = users(&call.args) else {
        return bad_args(&call, "users must be 1 to 32 Telegram user ids");
    };
    Planned::Later(Box::pin(async move {
        let seen = match seen(&call, &users).await {
            Ok(seen) => seen,
            Err(detail) => {
                return Done::from(call.answers.say("failed", &[("detail", &detail)], true));
            }
        };
        let (names, strangers) = named(&users, &call.names, &seen);
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

/// 没缓存名字的那几个号在这个群里说过话的（号 → 记下的名字，没洗）。
async fn seen(call: &Call, users: &[Id]) -> Result<HashMap<Id, Option<String>>, String> {
    let mut seen = HashMap::new();
    for user in users.iter().filter(|user| !call.names.contains_key(user)) {
        let query = HistoryQuery {
            text: String::new(),
            venue: Some(call.venue.clone()),
            sender: Some(user.to_string()),
            from: None,
            to: None,
            limit: 1,
        };
        if let Some(one) = call.history.search(query).await?.first() {
            seen.insert(user.clone(), one["name"].as_str().map(str::to_string));
        }
    }
    Ok(seen)
}
