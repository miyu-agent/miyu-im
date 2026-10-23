//! 读聊天记录（施工 C-1 中，设计 04 第一节第 3 条）：消息记录页一页一页往前翻、跨场所搜。纯 SQL，连接由调的一方开。
//!
//! - 一页（[`page`]）：一个场所的消息，从新到旧取 `limit` 条（`before` 是上一页最早那一条的行号），带上这几条的判断，和落在
//!   这段时间里的群事件；交回时照时刻从旧到新排好。
//! - 有记录的场所（[`venues`]）：每个场所最后一条的时刻、一共几条，新的在前。
//! - 搜（[`search`]）：关键词三个字以上照全文索引（`trigram`），一两个字照子串；能按场所、按人（号或名字）、按时间段筛；新的在前。

use rusqlite::{Connection, Row, params_from_iter, types::Value as Sql};
use serde_json::{Value, json};

/// 一页、一次搜最多几条。
pub(crate) const MOST: i64 = 200;

/// 消息表要读的几格（照 [`message`] 的先后）。
const COLUMNS: &str = "m.id, m.venue, m.msg, m.sender, m.name, m.at, m.text, m.reply_to, m.mentions, m.media, m.mine, m.recalled_at, m.recalled_by";

/// 读一行消息。
fn message(row: &Row<'_>) -> rusqlite::Result<Value> {
    let json_of = |text: String| serde_json::from_str::<Value>(&text).unwrap_or(json!([]));
    Ok(json!({
        "type": "message",
        "id": row.get::<_, i64>(0)?,
        "venue": row.get::<_, String>(1)?,
        "msg": row.get::<_, String>(2)?,
        "sender": row.get::<_, String>(3)?,
        "name": row.get::<_, Option<String>>(4)?,
        "at": row.get::<_, i64>(5)?,
        "text": row.get::<_, String>(6)?,
        "reply_to": row.get::<_, Option<String>>(7)?,
        "mentions": json_of(row.get(8)?),
        "media": json_of(row.get(9)?),
        "mine": row.get::<_, bool>(10)?,
        "recalled_at": row.get::<_, Option<i64>>(11)?,
        "recalled_by": row.get::<_, Option<String>>(12)?,
    }))
}

/// 场所 `venue` 的一页：比行号 `before` 早的（空的是最新的）最多 `limit` 条消息，带判断和这段时间的群事件。交回
/// `{items, more, before}`：`items` 从旧到新，`before` 是下一页从哪翻（这一页最早那一条的行号）。
///
/// # Errors
///
/// 库读不出。
pub(crate) fn page(
    connection: &Connection,
    venue: &str,
    before: Option<i64>,
    limit: i64,
) -> rusqlite::Result<Value> {
    let limit = limit.clamp(1, MOST);
    let mut statement = connection.prepare(&format!(
        "SELECT {COLUMNS} FROM messages m WHERE m.venue = ?1 AND m.id < ?2 ORDER BY m.id DESC LIMIT ?3"
    ))?;
    let mut messages: Vec<Value> = statement
        .query_map(
            params_from_iter([
                Sql::from(venue.to_string()),
                Sql::from(before.unwrap_or(i64::MAX)),
                Sql::from(limit),
            ]),
            message,
        )?
        .collect::<rusqlite::Result<_>>()?;
    let more = i64::try_from(messages.len()).unwrap_or(i64::MAX) == limit;
    let oldest_id = messages.last().and_then(|one| one["id"].as_i64());
    // 群事件取这一页的时间段：最早那一条以后，到下一页（更新的那一页）最早那一条以前；第一页到现在。
    let from = if more {
        messages
            .last()
            .and_then(|one| one["at"].as_i64())
            .unwrap_or(i64::MIN)
    } else {
        i64::MIN
    };
    let until = match before {
        Some(before) => connection
            .query_row("SELECT at FROM messages WHERE id = ?1", [before], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap_or(i64::MAX),
        None => i64::MAX,
    };
    let mut happenings = connection.prepare(
        "SELECT kind, at, target, actor, seconds, msg FROM happenings WHERE venue = ?1 AND at >= ?2 AND at < ?3 ORDER BY at",
    )?;
    let events: Vec<Value> = happenings
        .query_map(
            params_from_iter([
                Sql::from(venue.to_string()),
                Sql::from(from),
                Sql::from(until),
            ]),
            |row| {
                Ok(json!({
                    "type": "event",
                    "kind": row.get::<_, String>(0)?,
                    "at": row.get::<_, i64>(1)?,
                    "target": row.get::<_, Option<String>>(2)?,
                    "actor": row.get::<_, Option<String>>(3)?,
                    "seconds": row.get::<_, Option<i64>>(4)?,
                    "msg": row.get::<_, Option<String>>(5)?,
                }))
            },
        )?
        .collect::<rusqlite::Result<_>>()?;
    judged(connection, venue, &mut messages)?;
    let mut items: Vec<Value> = messages.into_iter().chain(events).collect();
    // 照时刻排，同一刻的照记下的先后（行号）；群事件排在同一刻的消息后面。
    items.sort_by_key(|item| {
        (
            item["at"].as_i64().unwrap_or_default(),
            item["id"].as_i64().unwrap_or(i64::MAX),
        )
    });
    Ok(json!({"items": items, "more": more, "before": oldest_id}))
}

/// 有记录的场所：交回 `{venues: [{venue, last, count}]}`，`last` 是最后一条的时刻，最近说过话的在前。
///
/// # Errors
///
/// 库读不出。
pub(crate) fn venues(connection: &Connection) -> rusqlite::Result<Value> {
    let mut statement = connection.prepare(
        "SELECT venue, MAX(at), COUNT(*) FROM messages GROUP BY venue ORDER BY MAX(at) DESC, venue",
    )?;
    let venues: Vec<Value> = statement
        .query_map([], |row| {
            Ok(json!({
                "venue": row.get::<_, String>(0)?,
                "last": row.get::<_, i64>(1)?,
                "count": row.get::<_, i64>(2)?,
            }))
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(json!({"venues": venues}))
}

/// 给场所 `venue` 的几条消息 `messages` 带上判断（`judged: {outcome, why}`，没判过的不带）。
fn judged(connection: &Connection, venue: &str, messages: &mut [Value]) -> rusqlite::Result<()> {
    let mut statement =
        connection.prepare("SELECT outcome, why FROM judgments WHERE venue = ?1 AND msg = ?2")?;
    for one in messages.iter_mut() {
        let Some(msg) = one["msg"].as_str().map(str::to_string) else {
            continue;
        };
        let found = statement.query_row([venue, msg.as_str()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        });
        if let Ok((outcome, why)) = found {
            let why = serde_json::from_str::<Value>(&why).unwrap_or(Value::Null);
            one["judged"] = json!({"outcome": outcome, "why": why});
        }
    }
    Ok(())
}

/// 搜的条件。
#[derive(Debug, Clone, Default)]
pub(crate) struct Query {
    /// 关键词：空的不按字筛。
    pub(crate) text: String,
    /// 只在这个场所里搜：空的跨场所。
    pub(crate) venue: Option<String>,
    /// 谁：全是数字的照号（平台身份的最后一段），别的照名字的子串。
    pub(crate) sender: Option<String>,
    /// 从、到（毫秒的 Unix 时间，含头不含尾）。
    pub(crate) from: Option<i64>,
    pub(crate) to: Option<i64>,
    /// 最多几条。
    pub(crate) limit: i64,
}

/// 照 `query` 搜（模块的说明）：新的在前，交回 `{items}`。
///
/// # Errors
///
/// 库读不出。
pub(crate) fn search(connection: &Connection, query: &Query) -> rusqlite::Result<Value> {
    let mut clauses: Vec<String> = Vec::new();
    let mut values: Vec<Sql> = Vec::new();
    let text = query.text.trim();
    let from_fts = text.chars().count() >= 3;
    let source = if from_fts {
        clauses.push("messages_text MATCH ?".to_string());
        values.push(Sql::from(format!("\"{}\"", text.replace('"', "\"\""))));
        "messages_text t JOIN messages m ON m.id = t.rowid"
    } else {
        if !text.is_empty() {
            clauses.push("m.text LIKE ? ESCAPE '\\'".to_string());
            values.push(Sql::from(format!("%{}%", like(text))));
        }
        "messages m"
    };
    if let Some(venue) = &query.venue {
        clauses.push("m.venue = ?".to_string());
        values.push(Sql::from(venue.clone()));
    }
    if let Some(sender) = query
        .sender
        .as_deref()
        .map(str::trim)
        .filter(|sender| !sender.is_empty())
    {
        if sender.bytes().all(|byte| byte.is_ascii_digit()) {
            clauses.push("m.sender LIKE ?".to_string());
            values.push(Sql::from(format!("%:{sender}")));
        } else {
            clauses.push("m.name LIKE ? ESCAPE '\\'".to_string());
            values.push(Sql::from(format!("%{}%", like(sender))));
        }
    }
    if let Some(from) = query.from {
        clauses.push("m.at >= ?".to_string());
        values.push(Sql::from(from));
    }
    if let Some(to) = query.to {
        clauses.push("m.at < ?".to_string());
        values.push(Sql::from(to));
    }
    let filter = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    values.push(Sql::from(query.limit.clamp(1, MOST)));
    let mut statement = connection.prepare(&format!(
        "SELECT {COLUMNS} FROM {source} {filter} ORDER BY m.at DESC, m.id DESC LIMIT ?"
    ))?;
    let items: Vec<Value> = statement
        .query_map(params_from_iter(values), message)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(json!({"items": items}))
}

/// `LIKE` 里的 `%`、`_`、`\` 照字面。
fn like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
