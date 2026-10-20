//! 群成员的名字（`onebot.md` 第一条「群消息」第 5 条，施工 O-22）：正文里的 @ 写成 `@名字`，名字照这里的缓存。NapCat 的 `at`
//! 段只有号，名字得另取（「施工时定的」第 59 条）：每条群消息的发的人顺手记下，缓存里没有的经 `get_group_member_info` 问。
//!
//! 按群、按号记，记多久照 `bridge.json` 的 `member_names_seconds`；过了的不用。时刻由用的一方交进来，测试不用等。
//!
//! 施工 O-31 另记身份（群主、管理员、普通成员，「平台工具（一）」第 7 条）：禁言不动群主和管理员。群消息的 `sender.role`、
//! `get_group_member_info` 回的 `role` 都带，同一个时限（「施工时定的」第 184 条）。

use serde_json::{Value, json};

use miyu_im::chat::{CallError, Link, Member};
pub use miyu_im::chat::{Members, Rank};

/// 问一个群成员的动作。
pub const MEMBER_INFO: &str = "get_group_member_info";

/// `get_group_member_info` 的参数：群 `group` 里号是 `user` 的人，让 NapCat 用它自己的缓存（`no_cache: false`）。
pub fn member_info(group: i64, user: i64) -> Value {
    json!({"group_id": group, "user_id": user, "no_cache": false})
}

/// `get_group_member_info` 的参数，不用 NapCat 的缓存（`no_cache: true`，施工 I-17：判她是不是管理员、踢人前后看人在不在群里，
/// 缓存会把刚撤的管理员、刚走的人照旧报回来）。
pub fn fresh_member_info(group: i64, user: i64) -> Value {
    json!({"group_id": group, "user_id": user, "no_cache": true})
}

/// 群 `group` 里号是 `user` 的人，现问（[`fresh_member_info`]）：回了成员的是他；回成了却没有这个人的是空的。
///
/// # Errors
///
/// 等不到、断了；回了失败（[`CallError::Failed`]：NapCat 对不在群里的人也回失败，要不要当「不在」由用的一方定）。
pub async fn fresh_member(
    link: &dyn Link,
    group: i64,
    user: i64,
) -> Result<Option<Member>, CallError> {
    let reply = link
        .raw(MEMBER_INFO, fresh_member_info(group, user))
        .await?;
    if reply["status"] != "ok" {
        return Err(super::failed(&reply));
    }
    Ok(super::replies::member(&reply["data"]).map(|mut member| {
        member.user = user;
        member
    }))
}

/// 一个人在群里叫什么：`who` 是群消息的 `sender`，或者 `get_group_member_info` 回的 `data`。`card`（群名片）去掉首尾
/// 空白不空的照它，不然 `nickname`（昵称，同样不空的）；都没有的是空的。照原样，没洗。
pub fn display_name(who: &Value) -> Option<String> {
    ["card", "nickname"]
        .iter()
        .filter_map(|key| who[*key].as_str())
        .find(|name| !name.trim().is_empty())
        .map(str::to_string)
}

/// 一个人在群里的身份（施工 O-31）：`who` 是群消息的 `sender`，或者 `get_group_member_info` 回的 `data`；`role` 是 `owner`、
/// `admin`、`member` 之一的才认，别的、没有的是空的。
pub fn rank_of(who: &Value) -> Option<Rank> {
    match who["role"].as_str()? {
        "owner" => Some(Rank::Owner),
        "admin" => Some(Rank::Admin),
        "member" => Some(Rank::Member),
        _ => None,
    }
}
