//! `miyu im access`（施工 I-16；2026-10-20 项目主人定：增删跳过判断名单「就是 access 工具的活」，照 I-13 白名单那样做成命令行，
//! 终端里的 AI 用 shell 调）：看、改谁能叫 AI 的几份名单。
//!
//! - `access`：三份都列出来（私聊白名单、名单里的群、各群跳过主动回复判断的人）。
//! - `access whitelist [list | add <号> | remove <号>]`：同 `miyu im whitelist`（`crate::whitelist`）。
//! - `access group [list | add <群号> | remove <群号>]`：名单里的群（会话页的群卡），经核心的 `package.call` 调桥自己的
//!   `cards.add`、`cards.remove`。
//! - `access skip <群号> [list | add <号> | remove <号>]`：那个群卡上的 `rules."chatty.skip_users"`（一项 `<平台>:<号>`），经
//!   `cards.get`、`cards.save_one` 读写。群不在名单里的说 [`Said::NoGroup`]、退出码 2。
//!
//! 号写错、连不上核心、核心拒了：同 `whitelist`。

use std::io::Write;

use serde_json::{Value, json};

use crate::PACKAGE;
use crate::chat::Platform;
use crate::texts::Texts;
use crate::whitelist::{self, Change};

/// 跳过判断名单在卡上的键。
const SKIP: &str = "chatty.skip_users";

/// 用法不对。
const USAGE: u8 = 2;

/// 连不上核心、核心拒了。
const FAILED: u8 = 1;

/// 要做的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    /// 三份都列出来。
    List,
    /// 私聊白名单。
    Whitelist(Change),
    /// 名单里的群。
    Group(Change),
    /// 这个群（群号照原样）跳过判断的人。
    Skip(String, Change),
}

/// 说给人听的几句（「给人看的字」`access/`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Said {
    /// 群不在名单里：群号。
    NoGroup(String),
    /// 号写得不对：照原样。
    Bad(String),
    /// 名单（名字）是空的。
    Empty(String),
    /// 加进了名单（名字）：那一项。
    Added(String, String),
    /// 已经在名单（名字）里：那一项。
    Already(String, String),
    /// 从名单（名字）去掉了：那一项。
    Removed(String, String),
    /// 名单（名字）里没有：那一项。
    Absent(String, String),
    /// 列出来时一份名单的头一行：名字。
    Heading(String),
}

/// 照 `access` 看、改：经连上了的 `core`，平台是 `platform`，照 `texts` 说；交回退出码。
pub async fn run(
    core: &mut miyu_client::open::Core,
    (access, platform): (&Access, Platform),
    texts: &Texts,
    (out, err): (&mut impl Write, &mut impl Write),
) -> u8 {
    match access {
        Access::Whitelist(change) => {
            whitelist::run(core, (change, platform), texts, (out, err)).await
        }
        Access::Group(change) => group(core, change, texts, (out, err)).await,
        Access::Skip(number, change) => {
            skip(core, (number, change, platform), texts, (out, err)).await
        }
        Access::List => {
            let code = whitelist::run(core, (&Change::List, platform), texts, (out, err)).await;
            if code != 0 {
                return code;
            }
            let cards = match bridge(core, "cards.get", json!({})).await {
                Ok(cards) => cards,
                Err(refused) => return said(err, &refused, FAILED),
            };
            let mut lines = vec![texts.access(&Said::Heading(texts.access_list("groups", "")))];
            for (id, set) in groups(&cards) {
                lines.push(id.clone());
                let skipped = skipped(set);
                if !skipped.is_empty() {
                    let name = texts.access_list("skip", &id);
                    let heading = texts.access(&Said::Heading(name));
                    lines.push(format!("  {heading} {}", skipped.join(", ")));
                }
            }
            said(out, &lines.join("\n"), 0)
        }
    }
}

/// 名单里的群：列、加、去。
async fn group(
    core: &mut miyu_client::open::Core,
    change: &Change,
    texts: &Texts,
    (out, err): (&mut impl Write, &mut impl Write),
) -> u8 {
    let cards = match bridge(core, "cards.get", json!({})).await {
        Ok(cards) => cards,
        Err(refused) => return said(err, &refused, FAILED),
    };
    let listed: Vec<String> = groups(&cards).into_iter().map(|(id, _)| id).collect();
    let name = texts.access_list("groups", "");
    let number = match change {
        Change::List if listed.is_empty() => {
            return said(out, &texts.access(&Said::Empty(name)), 0);
        }
        Change::List => return said(out, &listed.join("\n"), 0),
        Change::Add(number) | Change::Remove(number) => match number.trim().parse::<u64>() {
            Ok(parsed) if parsed > 0 => parsed.to_string(),
            _ => return said(err, &texts.access(&Said::Bad(number.clone())), USAGE),
        },
    };
    let here = listed.contains(&number);
    let (method, params, done) = match change {
        Change::Add(_) if here => return said(out, &texts.access(&Said::Already(name, number)), 0),
        Change::Remove(_) if !here => {
            return said(out, &texts.access(&Said::Absent(name, number)), 0);
        }
        Change::Add(_) => (
            "cards.add",
            json!({"value": number}),
            Said::Added(name, number.clone()),
        ),
        _ => (
            "cards.remove",
            json!({"card": {"kind": "group", "id": card_id(&number)}}),
            Said::Removed(name, number.clone()),
        ),
    };
    match bridge(core, method, params).await {
        Ok(_) => said(out, &texts.access(&done), 0),
        Err(refused) => said(err, &refused, FAILED),
    }
}

/// 一个群跳过判断的人：列、加、去。
async fn skip(
    core: &mut miyu_client::open::Core,
    (number, change, platform): (&str, &Change, Platform),
    texts: &Texts,
    (out, err): (&mut impl Write, &mut impl Write),
) -> u8 {
    let cards = match bridge(core, "cards.get", json!({})).await {
        Ok(cards) => cards,
        Err(refused) => return said(err, &refused, FAILED),
    };
    let group = number.trim().to_string();
    let Some((_, set)) = groups(&cards).into_iter().find(|(id, _)| *id == group) else {
        return said(err, &texts.access(&Said::NoGroup(group)), USAGE);
    };
    let listed = skipped(set);
    let name = texts.access_list("skip", &group);
    let (list, told) = match whitelist::changed(&listed, change, platform) {
        Ok(done) => done,
        Err(whitelist::Said::Bad(bad)) => return said(err, &texts.access(&Said::Bad(bad)), USAGE),
        Err(_) => return USAGE,
    };
    if let Some(list) = &list {
        let params = json!({"card": {"kind": "group", "id": card_id(&group)}, "values": {"rules": {SKIP: list}}});
        if let Err(refused) = bridge(core, "cards.save_one", params).await {
            return said(err, &refused, FAILED);
        }
    }
    let mut lines: Vec<String> = match change {
        Change::List => listed,
        _ => Vec::new(),
    };
    lines.extend(told.map(|told| texts.access(&from_whitelist(told, &name))));
    said(out, &lines.join("\n"), 0)
}

/// 卡片方法认的群卡编号：数（`cards.get` 交回的是数）；写不成数的照原样交，桥会拒。
fn card_id(group: &str) -> Value {
    group
        .parse::<u64>()
        .map_or_else(|_| json!(group), |id| json!(id))
}

/// 白名单那几句换成这一份名单（名字 `name`）的说法。
fn from_whitelist(told: whitelist::Said, name: &str) -> Said {
    let name = name.to_string();
    match told {
        whitelist::Said::Empty => Said::Empty(name),
        whitelist::Said::Bad(bad) => Said::Bad(bad),
        whitelist::Said::Added(entry) => Said::Added(name, entry),
        whitelist::Said::Already(entry) => Said::Already(name, entry),
        whitelist::Said::Removed(entry) => Said::Removed(name, entry),
        whitelist::Said::Absent(entry) => Said::Absent(name, entry),
    }
}

/// `cards.get` 的回答里名单里的群：群号（字）和卡上的格。
pub fn groups(cards: &Value) -> Vec<(String, &Value)> {
    cards["cards"]["groups"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|card| {
            let id = match &card["id"] {
                Value::Number(number) => number.to_string(),
                Value::String(text) => text.clone(),
                _ => return None,
            };
            Some((id, &card["set"]))
        })
        .collect()
}

/// 一张群卡上跳过判断的人。
pub fn skipped(set: &Value) -> Vec<String> {
    set["rules"][SKIP]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

/// 经核心的 `package.call` 调桥自己的方法 `method`：交回桥的回答，拒了的交回原话。
async fn bridge(
    core: &mut miyu_client::open::Core,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    core.call(
        PACKAGE,
        "package.call",
        json!({"package": PACKAGE, "method": method, "params": params}),
    )
    .await
    .map_err(|refused| refused.to_string())
}

/// `text` 印到 `to`，末尾换行（空的不印），交回 `code`；印不出去的交回 [`FAILED`]。
fn said(to: &mut impl Write, text: &str, code: u8) -> u8 {
    if text.is_empty() {
        return code;
    }
    match writeln!(to, "{text}").and_then(|()| to.flush()) {
        Ok(()) => code,
        Err(_) => FAILED,
    }
}

#[cfg(test)]
mod tests;
