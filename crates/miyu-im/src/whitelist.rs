//! `miyu im whitelist [list | add <号> | remove <号>]`（施工 I-13；2026-10-20 项目主人定：改白名单不做给 AI 的工具，要有命令行，
//! 终端里的 AI 用 shell 调）：看、改系统配置的 `im.whitelist`（一项写成 `<平台>:<号>`，例如 `qq:10001`）。照终端的样子连核心
//! （核心没在跑就照 `start` 那样拉起，同 `venue show`），`config.get` 读、`config.set` 写进系统那一层；改了当场生效（桥照推来的配置换）。
//!
//! - `list`（不写也是它）：一项一行；空的说 [`Said::Empty`]。
//! - `add`：号要是正整数，写错的在标准错误上说 [`Said::Bad`]、退出码 2；已经在的说 [`Said::Already`]，不写；加了的说
//!   [`Said::Added`]。
//! - `remove`：同上，不在的说 [`Said::Absent`]；去掉了的说 [`Said::Removed`]。
//! - 连不上核心、核心拒了：标准错误上说原话，退出码 1。

use std::io::Write;

use serde_json::{Value, json};

use crate::PACKAGE;
use crate::chat::Platform;
use crate::texts::Texts;

/// 配置的键。
const KEY: &str = "im.whitelist";

/// 号写错了：用法不对。
const USAGE: u8 = 2;

/// 连不上核心、核心拒了。
const FAILED: u8 = 1;

/// 要做的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// 列出来。
    List,
    /// 加一个号（照原样，还没认）。
    Add(String),
    /// 去掉一个号（照原样，还没认）。
    Remove(String),
}

/// 说给人听的几句（「给人看的字」`whitelist/`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Said {
    /// 白名单是空的。
    Empty,
    /// 号写得不对：照原样。
    Bad(String),
    /// 加了：那一项。
    Added(String),
    /// 已经在：那一项。
    Already(String),
    /// 去掉了：那一项。
    Removed(String),
    /// 本来就不在：那一项。
    Absent(String),
}

/// 照 `change` 改白名单 `listed`（纯逻辑）：交回改完的（没改的是空的）和要说的那一句。号写错的交回 `Err`。
///
/// # Errors
///
/// 号不是正整数。
pub fn changed(
    listed: &[String],
    change: &Change,
    platform: Platform,
) -> Result<(Option<Vec<String>>, Option<Said>), Said> {
    let entry = |number: &str| -> Result<String, Said> {
        let number = number.trim();
        match number.parse::<u64>() {
            Ok(parsed) if parsed > 0 => Ok(format!("{}:{parsed}", platform.name())),
            _ => Err(Said::Bad(number.to_string())),
        }
    };
    match change {
        Change::List => Ok((None, listed.is_empty().then_some(Said::Empty))),
        Change::Add(number) => {
            let entry = entry(number)?;
            if listed.contains(&entry) {
                return Ok((None, Some(Said::Already(entry))));
            }
            let mut list = listed.to_vec();
            list.push(entry.clone());
            Ok((Some(list), Some(Said::Added(entry))))
        }
        Change::Remove(number) => {
            let entry = entry(number)?;
            if !listed.contains(&entry) {
                return Ok((None, Some(Said::Absent(entry))));
            }
            let list = listed
                .iter()
                .filter(|one| **one != entry)
                .cloned()
                .collect();
            Ok((Some(list), Some(Said::Removed(entry))))
        }
    }
}

/// 照 `change` 看、改白名单：经连上了的 `core` 读写，平台是 `platform`，照 `texts` 说；交回退出码（印不出去的是 1）。
pub async fn run(
    core: &mut miyu_client::open::Core,
    (change, platform): (&Change, Platform),
    texts: &Texts,
    (out, err): (&mut impl Write, &mut impl Write),
) -> u8 {
    let listed: Vec<String> = match core
        .call(PACKAGE, "config.get", json!({"keys": [KEY]}))
        .await
    {
        Ok(answered) => answered["items"][KEY]["value"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        Err(refused) => return said(err, &refused.to_string(), FAILED),
    };
    let (list, told) = match changed(&listed, change, platform) {
        Ok(done) => done,
        Err(bad) => return said(err, &texts.whitelist(&bad), USAGE),
    };
    if let Some(list) = &list {
        let changes = json!([{"key": KEY, "value": list}]);
        let wrote = core
            .call(
                PACKAGE,
                "config.set",
                json!({"layer": "system", "changes": changes}),
            )
            .await;
        if let Err(refused) = wrote {
            return said(err, &refused.to_string(), FAILED);
        }
    }
    let mut lines: Vec<String> = match change {
        Change::List => listed,
        _ => Vec::new(),
    };
    lines.extend(told.map(|told| texts.whitelist(&told)));
    said(out, &lines.join("\n"), 0)
}

/// `text` 印到 `to`，末尾换行（空的不印），交回 `code`；印不出去的交回 [`FAILED`]（同 `venue show`）。
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
