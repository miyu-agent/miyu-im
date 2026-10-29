//! 通讯录（施工 I-12，设计 18 第十一节「通讯录」：发消息时照名字找人）：`qq_contacts {kind?, query?}` 列她的好友（`friends`，出厂）
//! 或者她在的群（`groups`），`query` 照名字、号筛，不分大小写；最多列 `members_shown` 个（`bridge.json`，同群成员）。一个一段
//! `<号> <名字>`，`; ` 隔开（模板换字会转义换行）。私聊、群里都给。

use miyu_im::chat::Id;
use serde_json::Value;

use miyu_im::toolset::{Call, NAME, Planned, clean};

use super::args::{bad_args, optional_text};
use super::lookup::{Undone, said, undone};

/// 列哪一种。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// 好友。
    Friends,
    /// 群。
    Groups,
}

/// 读 `kind`（纯逻辑）：没写的是好友；写错的是空的。
pub(crate) fn kind(args: &Value) -> Option<Kind> {
    match args["kind"].as_str() {
        None | Some("friends") => Some(Kind::Friends),
        Some("groups") => Some(Kind::Groups),
        Some(_) => None,
    }
}

/// 照 `query` 筛、列成一段（纯逻辑）：交回（列的几个、一共几个、那一段）。名字洗过，没有的空着。
pub(crate) fn listed(
    people: &[(Id, Option<String>)],
    query: Option<&str>,
    shown: usize,
) -> (usize, usize, String) {
    let query = query
        .map(|query| query.trim().to_lowercase())
        .filter(|query| !query.is_empty());
    let found: Vec<String> = people
        .iter()
        .filter_map(|(id, name)| {
            let name = name
                .as_deref()
                .and_then(|name| clean(name, NAME))
                .unwrap_or_default();
            if let Some(query) = &query
                && !name.to_lowercase().contains(query.as_str())
                && !id.to_string().contains(query.as_str())
            {
                return None;
            }
            Some(format!("{id} {name}").trim_end().to_string())
        })
        .collect();
    let total = found.len();
    let list: Vec<String> = found.into_iter().take(shown).collect();
    (list.len(), total, list.join("; "))
}

/// `qq_contacts` 的一次调用（模块的说明）。
pub(super) fn contacts(shown: usize, call: Call) -> Planned {
    // 施工 I-17：写错的说哪个参数。
    let Some(kind) = kind(&call.args) else {
        return bad_args(&call, "kind must be friends or groups");
    };
    if let Err(bad) = optional_text(&call.args, "query", "query must be text") {
        return bad_args(&call, bad);
    }
    Planned::later(async move {
        match listing(&call, kind, shown).await {
            Ok(answer) => answer,
            Err(undone) => said(&call, undone),
        }
    })
}

/// 问 QQ、列出来。
async fn listing(call: &Call, kind: Kind, shown: usize) -> Result<Value, Undone> {
    let Some(link) = &call.link else {
        return Err(("unreachable", None));
    };
    let people: Vec<(Id, Option<String>)> = match kind {
        Kind::Friends => link
            .friends()
            .await
            .map_err(undone)?
            .into_iter()
            .map(|one| (one.id, one.name))
            .collect(),
        Kind::Groups => link
            .groups()
            .await
            .map_err(undone)?
            .into_iter()
            .map(|one| (one.id, one.name))
            .collect(),
    };
    let (count, total, list) = listed(&people, call.args["query"].as_str(), shown);
    if count == 0 {
        return Ok(call.answers.say("no-contacts", &[], false));
    }
    let (count, total) = (count.to_string(), total.to_string());
    let kind = match kind {
        Kind::Friends => "friends",
        Kind::Groups => "groups",
    };
    Ok(call.answers.say(
        "contacts",
        &[
            ("shown", &count),
            ("total", &total),
            ("kind", kind),
            ("list", &list),
        ],
        false,
    ))
}

#[cfg(test)]
mod tests;
