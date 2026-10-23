//! 删聊天记录（施工 C-1 中，设计 04 第一节第 2 条；项目主人 10-23 定：只在 im 后台的消息记录页里删，AI 不许删，能批量）：按场所
//! （一次几个）、时间段、人删，或者全部删。删的只是这个库：她当时看过的还在她的会话里，她的记忆也不跟着删。
//!
//! 删消息的同时删掉它们的判断；按场所、时间段删（不按人）的连这段时间的群事件一起删。

use rusqlite::{Connection, params_from_iter, types::Value as Sql};

/// 删哪些：几样都空着的是什么都不删（全部删要写 `all`）。
#[derive(Debug, Clone, Default)]
pub(crate) struct Scope {
    /// 只删这几个场所的：空的不按场所筛。
    pub(crate) venues: Vec<String>,
    /// 从、到（毫秒的 Unix 时间，含头不含尾）。
    pub(crate) from: Option<i64>,
    pub(crate) to: Option<i64>,
    /// 只删这个人（平台身份）说的。
    pub(crate) sender: Option<String>,
    /// 全部删。
    pub(crate) all: bool,
}

/// 照 `scope` 删（模块的说明）：交回删了几条消息。什么条件都没写、又没写 `all` 的不删、交回 0。
///
/// # Errors
///
/// 库写不进。
pub(crate) fn delete(connection: &mut Connection, scope: &Scope) -> rusqlite::Result<usize> {
    let mut clauses: Vec<&str> = Vec::new();
    let mut values: Vec<Sql> = Vec::new();
    let marks = vec!["?"; scope.venues.len()].join(", ");
    let in_venues = format!("venue IN ({marks})");
    if !scope.venues.is_empty() {
        clauses.push(&in_venues);
        values.extend(scope.venues.iter().cloned().map(Sql::from));
    }
    if let Some(from) = scope.from {
        clauses.push("at >= ?");
        values.push(Sql::from(from));
    }
    if let Some(to) = scope.to {
        clauses.push("at < ?");
        values.push(Sql::from(to));
    }
    let event_values = values.clone();
    let event_clauses = clauses.clone();
    if let Some(sender) = &scope.sender {
        clauses.push("sender = ?");
        values.push(Sql::from(sender.clone()));
    }
    if clauses.is_empty() && !scope.all {
        return Ok(0);
    }
    let filter = |clauses: &[&str]| {
        if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        }
    };
    let transaction = connection.transaction()?;
    let deleted = transaction.execute(
        &format!("DELETE FROM messages {}", filter(&clauses)),
        params_from_iter(values),
    )?;
    if scope.sender.is_none() {
        transaction.execute(
            &format!("DELETE FROM happenings {}", filter(&event_clauses)),
            params_from_iter(event_values),
        )?;
    }
    transaction.execute(
        "DELETE FROM judgments WHERE NOT EXISTS (SELECT 1 FROM messages m WHERE m.venue = judgments.venue AND m.msg = judgments.msg)",
        [],
    )?;
    transaction.commit()?;
    Ok(deleted)
}
