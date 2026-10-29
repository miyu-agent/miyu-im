//! 头像和群成员（施工 O-54，`onebot.md` 第一条「头像和群成员」，2026-10-17 项目主人定；施工 T-1 从宿主挪进驱动）：两件工具，im 照
//! 会话填好场景交过来，认参数，要去 QQ 上办的交给 im 另起任务等着（跟核心的那一头不等）。
//!
//! - `avatar`：一个人的（`user`）、一个群的（`group`）头像，照 `bridge.json` 的地址（`avatar_user_url`、`avatar_group_url`，
//!   `{id}` 换成号）下下来，存进这个会话的工作区 `qq-files/avatar-qq-<号>.<扩展名>`、`avatar-group-<号>.<…>`，答相对工作区的
//!   路径（同 `fetch_media`）；她看不看图、拿它做什么是她的事。两个都没写的取叫她的人（`by.id`；私聊里是对方）。
//! - `members`：这个群的成员（`get_group_member_list`），`query` 写了的只留名字（群名片、昵称）含着它的，最多列 `limit` 个（施工
//!   I-17：没写的照 `members_shown`，最多 [`MEMBERS_MOST`]，照旧版），一行一个：QQ 号、名字、身份、头衔。

use std::time::Duration;

use serde_json::Value;

use miyu_im::chat::{CallError, Id, Member, Rank, Source, To};
use miyu_im::toolset::{Call, NAME, Planned, QQ_FILES, bytes, clean, image_extension, save};

use super::QqTools;
use super::args::{Bad, bad_args, group_only, optional_id, optional_number, optional_text};

/// 答的话里的原因最多几个字符（同取图）。
const DETAIL: usize = 200;

/// 群成员一次最多列几个（施工 I-17，同旧版 `get_group_members_info` 的上限）。
pub(crate) const MEMBERS_MOST: usize = 200;

/// 要谁的头像（纯逻辑）：她给的群号、QQ 号（正整数，写成字的也认），都没给的取叫她的人。两个都给了的、写错的是空的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Whose {
    /// 一个人。
    User(Id),
    /// 一个群。
    Group(Id),
}

/// 读 `avatar` 的参数 `args`，叫她的人是 `caller`（号；本机的、不知道的是空的）。
pub(crate) fn whose(args: &Value, caller: Option<Id>) -> Option<Whose> {
    let number = |value: &Value| match value {
        Value::Null => Some(None),
        Value::Number(number) => number
            .as_i64()
            .filter(|n| *n > 0)
            .map(|n| Some(Id::from(n))),
        Value::String(text) => text
            .trim()
            .parse::<i64>()
            .ok()
            .filter(|n| *n > 0)
            .map(|n| Some(Id::from(n))),
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
/// 有头衔的写成 `(<身份>, title: <头衔>)`（施工 I-17，洗过）；`; ` 隔开；最多 `shown` 个。
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
            let user = member.user.clone();
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
            let title = member.title.as_deref().and_then(|title| clean(title, NAME));
            Some(match title {
                Some(title) => format!("{user} {name} ({role}, title: {title})"),
                None => format!("{user} {name} ({role})"),
            })
        })
        .collect();
    let total = found.len();
    let list: Vec<String> = found.into_iter().take(shown).collect();
    (list.len(), total, list.join("; "))
}

/// `avatar`：认要谁的（参数写错的答写错了哪个，施工 I-17；没写、又不知道谁叫的当场答 `no-target`），下下来存进工作区。
pub(super) fn avatar(tools: &QqTools, call: Call) -> Planned {
    let given = (
        optional_id(&call.args, "user", "user must be a QQ number"),
        optional_id(&call.args, "group", "group must be a group number"),
    );
    match given {
        (Err(bad), _) | (_, Err(bad)) => return bad_args(&call, bad),
        (Ok(Some(_)), Ok(Some(_))) => return bad_args(&call, "give user or group, not both"),
        _ => {}
    }
    let Some(whose) = whose(&call.args, call.caller.clone()) else {
        return Planned::Now(call.answers.say("no-target", &[], true));
    };
    let (url, name) = avatar_of(whose, &tools.avatars);
    let wait = (tools.wait, tools.image_limit);
    Planned::later(async move {
        match fetched(&url, &name, (&call.cwd, std::env::home_dir()), wait).await {
            Ok(path) => call.answers.say("saved", &[("path", &path)], false),
            Err(undone) => said(&call, undone),
        }
    })
}

/// `members`：列这个群的成员（只给群：私聊里调到的照说不会，答只给群）。`limit` 写错的、`query` 不是字的答写错了哪个。
pub(super) fn members(tools: &QqTools, call: Call) -> Planned {
    let To::Group(group) = call.to.clone() else {
        return group_only(&call);
    };
    let shown = match limit(&call.args, tools.members) {
        Ok(shown) => shown,
        Err(bad) => return bad_args(&call, bad),
    };
    if let Err(bad) = optional_text(&call.args, "query", "query must be text") {
        return bad_args(&call, bad);
    }
    Planned::later(async move {
        match listing(&call, group, shown).await {
            Ok(answer) => answer,
            Err(undone) => said(&call, undone),
        }
    })
}

/// 列几个（纯逻辑）：`limit` 是 1 到 [`MEMBERS_MOST`] 的整数（写成字的也认），没写的照 `shown`（`members_shown`，超过上限的照上限）。
pub(crate) fn limit(args: &Value, shown: usize) -> Result<usize, Bad> {
    let bad = "limit must be 1 to 200";
    match optional_number(args, "limit", bad)? {
        None => Ok(shown.min(MEMBERS_MOST)),
        Some(limit) => usize::try_from(limit)
            .ok()
            .filter(|limit| *limit <= MEMBERS_MOST)
            .ok_or(bad),
    }
}

/// 没做成的那一句：带原因的照填。
pub(super) fn said(call: &Call, (why, detail): Undone) -> Value {
    let fields: Vec<(&str, &str)> = detail
        .as_deref()
        .map(|detail| ("detail", detail))
        .into_iter()
        .collect();
    call.answers.say(why, &fields, true)
}

/// 没做成的：那一句的名字、带的原因。
pub(super) type Undone = (&'static str, Option<String>);

/// 下 `url`（最多 `limit` 字节：头像照图的上限，群文件不设，施工 I-23）、照 `name` 加上看出来的扩展名存进工作区：交回相对工作区的
/// 路径（头像、群文件，施工 I-12 起共用）。
pub(super) async fn fetched(
    url: &str,
    name: &str,
    (cwd, home): (&str, Option<std::path::PathBuf>),
    (wait, limit): (Duration, usize),
) -> Result<String, Undone> {
    let got = bytes(&[Source::Url(url.to_string())], wait, limit)
        .await
        .map_err(|why| ("unfetched", Some(cut(&why))))?;
    // 名字自己带了扩展名的（群文件的原名）照它；没带的认得出是图的补上（头像），认不出的照原样（施工 I-16：以前一律补 `.jpg`）。
    let file = match (
        std::path::Path::new(name).extension(),
        image_extension(&got),
    ) {
        (None, Some(ext)) => format!("{name}.{ext}"),
        _ => name.to_string(),
    };
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
async fn listing(call: &Call, group: Id, shown: usize) -> Result<Value, Undone> {
    let Some(link) = &call.link else {
        return Err(("unreachable", None));
    };
    let list = link.members(&group).await.map_err(undone)?;
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

/// 平台没成算成哪一句（施工 I-12 起共用）：回了失败的是 `failed` 带原话，等不到是 `unanswered`，没连着是 `unreachable`。
pub(super) fn undone(error: CallError) -> Undone {
    match error {
        CallError::Failed { said, .. } => ("failed", Some(said)),
        CallError::Timeout => ("unanswered", None),
        CallError::Closed => ("unreachable", None),
        CallError::Unsupported => ("failed", Some("unsupported".to_string())),
    }
}

/// `members` 的 `query`：只留名字、号含着它的。
fn query(call: &Call) -> Option<&str> {
    call.args["query"].as_str()
}

/// 截到 [`DETAIL`] 个字符。
pub(super) fn cut(text: &str) -> String {
    text.chars().take(DETAIL).collect()
}

#[cfg(test)]
mod tests;
