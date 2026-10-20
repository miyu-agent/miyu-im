//! 头像和群成员（施工 O-54，`onebot.md` 第一条「头像和群成员」，2026-10-17 项目主人定；施工 T-1 从宿主挪进驱动）：两件工具，im 照
//! 会话填好场景交过来，认参数，要去 QQ 上办的交给 im 另起任务等着（跟核心的那一头不等）。
//!
//! - `avatar`：一个人的（`user`）、一个群的（`group`）头像，照 `bridge.json` 的地址（`avatar_user_url`、`avatar_group_url`，
//!   `{id}` 换成号）下下来，存进这个会话的工作区 `qq-files/avatar-qq-<号>.<扩展名>`、`avatar-group-<号>.<…>`，答相对工作区的
//!   路径（同 `fetch_media`）；她看不看图、拿它做什么是她的事。两个都没写的取叫她的人（`by.id`；私聊里是对方）。
//! - `members`：这个群的成员（`get_group_member_list`），`query` 写了的只留名字（群名片、昵称）含着它的，最多列
//!   `members_shown` 个，一行一个：QQ 号、名字、身份。

use std::time::Duration;

use serde_json::Value;

use miyu_im::chat::{CallError, Member, Rank, Source, To};
use miyu_im::toolset::{Call, NAME, Planned, QQ_FILES, bytes, clean, extension, save};

use super::QqTools;

/// 答的话里的原因最多几个字符（同取图）。
const DETAIL: usize = 200;

/// 要谁的头像（纯逻辑）：她给的群号、QQ 号（正整数，写成字的也认），都没给的取叫她的人。两个都给了的、写错的是空的。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Whose {
    /// 一个人。
    User(i64),
    /// 一个群。
    Group(i64),
}

/// 读 `avatar` 的参数 `args`，叫她的人是 `caller`（号；本机的、不知道的是空的）。
pub(crate) fn whose(args: &Value, caller: Option<i64>) -> Option<Whose> {
    let number = |value: &Value| match value {
        Value::Null => Some(None),
        Value::Number(number) => number.as_i64().filter(|n| *n > 0).map(Some),
        Value::String(text) => text.trim().parse::<i64>().ok().filter(|n| *n > 0).map(Some),
        _ => None,
    };
    match (number(&args["user"])?, number(&args["group"])?) {
        (Some(user), None) => Some(Whose::User(user)),
        (None, Some(group)) => Some(Whose::Group(group)),
        (None, None) => caller.map(Whose::User),
        (Some(_), Some(_)) => None,
    }
}

/// 头像的地址和存的文件名（不带扩展名）：照模板 `(一个人的, 一个群的)` 换号。
pub(crate) fn avatar_of(whose: Whose, templates: &(String, String)) -> (String, String) {
    match whose {
        Whose::User(user) => (
            templates.0.replace("{id}", &user.to_string()),
            format!("avatar-qq-{user}"),
        ),
        Whose::Group(group) => (
            templates.1.replace("{id}", &group.to_string()),
            format!("avatar-group-{group}"),
        ),
    }
}

/// 群成员列表 `data`（`get_group_member_list` 的回答）照 `query` 筛、列成一行（纯逻辑）：交回（列的几个、一共几个、那一行）。
/// 名字照群名片，空的照昵称（洗过）；`query` 照名字、号找，不分大小写；身份没写的当 `member`；一个一段 `<号> <名字> (<身份>)`，
/// `; ` 隔开；最多 `shown` 个。
pub(crate) fn listed(
    members: &[Member],
    query: Option<&str>,
    shown: usize,
) -> (usize, usize, String) {
    let query = query
        .map(|query| query.trim().to_lowercase())
        .filter(|query| !query.is_empty());
    let found: Vec<String> = members
        .iter()
        .filter_map(|member| {
            let user = member.user;
            let name = member
                .name
                .as_deref()
                .and_then(|name| clean(name, NAME))
                .unwrap_or_default();
            if let Some(query) = &query
                && !name.to_lowercase().contains(query.as_str())
                && !user.to_string().contains(query.as_str())
            {
                return None;
            }
            let role = member.rank.map_or("member", Rank::word);
            Some(format!("{user} {name} ({role})"))
        })
        .collect();
    let total = found.len();
    let list: Vec<String> = found.into_iter().take(shown).collect();
    (list.len(), total, list.join("; "))
}

/// `avatar`：认要谁的（认不出的当场答 `no-target`），下下来存进工作区。
pub(super) fn avatar(tools: &QqTools, call: Call) -> Planned {
    let Some(whose) = whose(&call.args, call.caller) else {
        return Planned::Now(call.answers.say("no-target", &[], true));
    };
    let (url, name) = avatar_of(whose, &tools.avatars);
    let wait = tools.wait;
    Planned::later(async move {
        match fetched(&url, &name, (&call.cwd, std::env::home_dir()), wait).await {
            Ok(path) => call.answers.say("saved", &[("path", &path)], false),
            Err(undone) => said(&call, undone),
        }
    })
}

/// `members`：列这个群的成员（只给群：私聊里调到的照说不会，答 `unreachable`）。
pub(super) fn members(tools: &QqTools, call: Call) -> Planned {
    let To::Group(group) = call.to else {
        return Planned::Now(call.answers.say("unreachable", &[], true));
    };
    let shown = tools.members;
    Planned::later(async move {
        match listing(&call, group, shown).await {
            Ok(answer) => answer,
            Err(undone) => said(&call, undone),
        }
    })
}

/// 没做成的那一句：带原因的照填。
fn said(call: &Call, (why, detail): Undone) -> Value {
    let fields: Vec<(&str, &str)> = detail
        .as_deref()
        .map(|detail| ("detail", detail))
        .into_iter()
        .collect();
    call.answers.say(why, &fields, true)
}

/// 没做成的：那一句的名字、带的原因。
type Undone = (&'static str, Option<String>);

/// 下头像、存进工作区：交回相对工作区的路径。
async fn fetched(
    url: &str,
    name: &str,
    (cwd, home): (&str, Option<std::path::PathBuf>),
    wait: Duration,
) -> Result<String, Undone> {
    let got = bytes(&[Source::Url(url.to_string())], wait)
        .await
        .map_err(|why| ("unfetched", Some(cut(&why))))?;
    let file = format!("{name}.{}", extension(&got));
    let cwd = cwd.to_string();
    let saved = tokio::task::spawn_blocking(move || save(&cwd, home.as_deref(), &file, &got))
        .await
        .map_err(|error| error.to_string())
        .and_then(|saved| saved);
    match saved {
        Ok(path) => {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            Ok(format!("{QQ_FILES}/{}", name.unwrap_or_default()))
        }
        Err(detail) => Err(("not-saved", Some(cut(&detail)))),
    }
}

/// 列群成员。
async fn listing(call: &Call, group: i64, shown: usize) -> Result<Value, Undone> {
    let Some(link) = &call.link else {
        return Err(("unreachable", None));
    };
    let list = link.members(group).await.map_err(|error| match error {
        CallError::Failed { said, .. } => ("failed", Some(said)),
        CallError::Timeout => ("unanswered", None),
        CallError::Closed => ("unreachable", None),
    })?;
    let (count, total, list) = listed(&list, query(call), shown);
    if count == 0 {
        return Ok(call.answers.say("no-members", &[], false));
    }
    let (count, total) = (count.to_string(), total.to_string());
    Ok(call.answers.say(
        "members",
        &[("shown", &count), ("total", &total), ("list", &list)],
        false,
    ))
}

/// `members` 的 `query`：只留名字、号含着它的。
fn query(call: &Call) -> Option<&str> {
    call.args["query"].as_str()
}

/// 截到 [`DETAIL`] 个字符。
fn cut(text: &str) -> String {
    text.chars().take(DETAIL).collect()
}

#[cfg(test)]
mod tests;
