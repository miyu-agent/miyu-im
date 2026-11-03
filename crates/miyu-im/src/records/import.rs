//! 导入聊天记录（施工 I-127，设计 04 第四节）：一份中立的 JSON Lines，一行一条消息、一件群里的事或者一条判断，字段照库里的列。
//! 不认哪个旧版的格式：旧库转成这种写法是迁移程序的事。
//!
//! - 先整份读、查：有一行不对的整份不导，交回每一行的问题（[`Rejected`]）。
//! - 再在一个事务里写：消息照 (场所, 平台编号) 去重，库里已经有的留着库里的；同一个编号、发的人或时刻不一样的另数一份
//!   （平台编号偶尔撞上），也留着库里的。群里的事一模一样的不再写；撤回的照样给那一条标上撤回（已经标了的不动）。判断照
//!   (场所, 平台编号) 去重，库里已经有的留着。所以重导、和现有的记录重叠都不会多出来。

use std::io::BufRead;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use serde_json::Value;

use miyu_chat::Venue;
use miyu_client::ids::VenueId;

use super::RECALLED;

/// 群里的事认的几种（同 [`super::Happening::kind`]）。
const KINDS: &[&str] = &["muted", "unmuted", "kicked", "joined", "left", RECALLED];

/// 判断认的几种结论（同 [`super::Write::Judged`]）。
const OUTCOMES: &[&str] = &["reply", "record", "deferred"];

/// 一行。
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
enum Line {
    /// 一条消息：`messages` 的一行。
    Message {
        venue: String,
        msg: String,
        sender: String,
        #[serde(default)]
        name: Option<String>,
        at: i64,
        text: String,
        #[serde(default)]
        reply_to: Option<String>,
        #[serde(default)]
        mentions: Vec<String>,
        #[serde(default)]
        media: Vec<Value>,
        #[serde(default)]
        mine: bool,
        #[serde(default)]
        recalled_at: Option<i64>,
        #[serde(default)]
        recalled_by: Option<String>,
    },
    /// 群里的一件事：`happenings` 的一行。
    Happening {
        venue: String,
        at: i64,
        kind: String,
        #[serde(default)]
        target: Option<String>,
        #[serde(default)]
        actor: Option<String>,
        #[serde(default)]
        seconds: Option<i64>,
        #[serde(default)]
        msg: Option<String>,
    },
    /// 一条消息的判断：`judgments` 的一行。
    Judgment {
        venue: String,
        msg: String,
        at: i64,
        outcome: String,
        why: Value,
    },
}

/// 一行哪里不对。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Problem {
    /// 不是这种写法的 JSON：解析器说的原话。
    Json(String),
    /// 场所编号不是带机器人号的新写法（`<平台>:<机器人号>:<种类>:<号>`）。
    Venue(String),
    /// 群里的事不认这一种。
    Kind(String),
    /// 判断不认这个结论。
    Outcome(String),
    /// 这一格不能是空的、不能是 0 或负数。
    Empty(&'static str),
}

/// 有不对的行，整份不导：第几行（从 1 数）、哪里不对。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rejected(pub(crate) Vec<(usize, Problem)>);

/// 导完了数一数。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Imported {
    /// 新写进去的消息、群里的事、判断。
    pub(crate) messages: usize,
    pub(crate) happenings: usize,
    pub(crate) judgments: usize,
    /// 库里已经有、一模一样的（消息照发的人和时刻比，群里的事照每一格比，判断照编号），没写。
    pub(crate) duplicates: usize,
    /// 同一个场所、同一个平台编号，发的人或时刻不一样的消息：留着库里的，没写。
    pub(crate) conflicts: usize,
}

/// 读出 `input` 的每一行，查过（模块的说明）。空行跳过。
///
/// # Errors
///
/// 有不对的行：交回每一行的问题。读不了的（不是 UTF-8、读的时候出错）算那一行的 [`Problem::Json`]。
fn parse(input: impl BufRead) -> Result<Vec<Line>, Rejected> {
    let mut lines = Vec::new();
    let mut problems = Vec::new();
    for (index, read) in input.lines().enumerate() {
        let number = index + 1;
        let text = match read {
            Ok(text) => text,
            Err(error) => {
                problems.push((number, Problem::Json(error.to_string())));
                continue;
            }
        };
        if text.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Line>(&text) {
            Ok(line) => match checked(&line) {
                Some(problem) => problems.push((number, problem)),
                None => lines.push(line),
            },
            Err(error) => problems.push((number, Problem::Json(error.to_string()))),
        }
    }
    if problems.is_empty() {
        Ok(lines)
    } else {
        Err(Rejected(problems))
    }
}

/// 一行解出来以后哪里不对，都对的是 `None`。
fn checked(line: &Line) -> Option<Problem> {
    let (venue, at) = match line {
        Line::Message { venue, at, .. }
        | Line::Happening { venue, at, .. }
        | Line::Judgment { venue, at, .. } => (venue, *at),
    };
    if !is_new_venue(venue) {
        return Some(Problem::Venue(venue.clone()));
    }
    if at <= 0 {
        return Some(Problem::Empty("at"));
    }
    match line {
        Line::Message { msg, .. } if msg.is_empty() => Some(Problem::Empty("msg")),
        Line::Message { sender, .. } if sender.is_empty() => Some(Problem::Empty("sender")),
        Line::Happening { kind, .. } if !KINDS.contains(&kind.as_str()) => {
            Some(Problem::Kind(kind.clone()))
        }
        Line::Judgment { msg, .. } if msg.is_empty() => Some(Problem::Empty("msg")),
        Line::Judgment { outcome, .. } if !OUTCOMES.contains(&outcome.as_str()) => {
            Some(Problem::Outcome(outcome.clone()))
        }
        _ => None,
    }
}

/// `venue` 是不是带机器人号的场所编号：老写法（`<平台>:<种类>:<号>`）im 解得出，但库里只放新写法。
fn is_new_venue(venue: &str) -> bool {
    let old = venue
        .split(':')
        .nth(1)
        .is_some_and(|second| matches!(second, "group" | "private"));
    !old && VenueId::parse(venue)
        .ok()
        .and_then(|id| Venue::parse(&id))
        .is_some()
}

/// 把 `input` 导进 `connection` 的库（模块的说明）：整份查过，在一个事务里写。
///
/// # Errors
///
/// 有不对的行（[`Rejected`]，什么都没写），或者库写不进（原话；事务退回，什么都没写）。
pub(crate) fn import(
    connection: &mut Connection,
    input: impl BufRead,
) -> Result<Result<Imported, Rejected>, rusqlite::Error> {
    let lines = match parse(input) {
        Ok(lines) => lines,
        Err(rejected) => return Ok(Err(rejected)),
    };
    let transaction = connection.transaction()?;
    let mut imported = Imported::default();
    for line in &lines {
        written(&transaction, line, &mut imported)?;
    }
    transaction.commit()?;
    Ok(Ok(imported))
}

/// 写一行，数进 `imported`。
fn written(connection: &Connection, line: &Line, imported: &mut Imported) -> rusqlite::Result<()> {
    match line {
        Line::Message {
            venue,
            msg,
            sender,
            name,
            at,
            text,
            reply_to,
            mentions,
            media,
            mine,
            recalled_at,
            recalled_by,
        } => {
            let there: Option<(String, i64)> = connection
                .query_row(
                    "SELECT sender, at FROM messages WHERE venue = ?1 AND msg = ?2",
                    params![venue, msg],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            match there {
                Some((was, when)) if was == *sender && when == *at => imported.duplicates += 1,
                Some(_) => imported.conflicts += 1,
                None => {
                    let mentions = serde_json::to_string(mentions).unwrap_or_else(|_| "[]".into());
                    let media = serde_json::to_string(media).unwrap_or_else(|_| "[]".into());
                    connection.execute(
                        "INSERT INTO messages (venue, msg, sender, name, at, text, reply_to, mentions, media, mine, recalled_at, recalled_by)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                        params![venue, msg, sender, name, at, text, reply_to, mentions, media, mine, recalled_at, recalled_by],
                    )?;
                    imported.messages += 1;
                }
            }
        }
        Line::Happening {
            venue,
            at,
            kind,
            target,
            actor,
            seconds,
            msg,
        } => {
            let there: bool = connection.query_row(
                "SELECT EXISTS (SELECT 1 FROM happenings WHERE venue = ?1 AND at = ?2 AND kind = ?3 AND target IS ?4
                 AND actor IS ?5 AND seconds IS ?6 AND msg IS ?7)",
                params![venue, at, kind, target, actor, seconds, msg],
                |row| row.get(0),
            )?;
            if there {
                imported.duplicates += 1;
            } else {
                connection.execute(
                    "INSERT INTO happenings (venue, at, kind, target, actor, seconds, msg) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![venue, at, kind, target, actor, seconds, msg],
                )?;
                imported.happenings += 1;
            }
            if let (RECALLED, Some(msg)) = (kind.as_str(), msg) {
                connection.execute(
                    "UPDATE messages SET recalled_at = ?1, recalled_by = ?2
                     WHERE venue = ?3 AND msg = ?4 AND recalled_at IS NULL",
                    params![at, actor, venue, msg],
                )?;
            }
        }
        Line::Judgment {
            venue,
            msg,
            at,
            outcome,
            why,
        } => {
            let added = connection.execute(
                "INSERT OR IGNORE INTO judgments (venue, msg, at, outcome, why) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![venue, msg, at, outcome, why.to_string()],
            )?;
            if added == 0 {
                imported.duplicates += 1;
            } else {
                imported.judgments += 1;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
