//! 引用的那一条是谁发的（施工 LW-IM-7，`onebot.md` 第一条「群里怎么叫她」第 1 条）：群的投影只留一个窗口，引用的那一条丢出
//! 窗口以后，照它的平台编号在聊天记录库里找发的人。纯 SQL，连接由调的一方开。
//!
//! 只认人说的（`mine` 是 0：引用她的话照旧算「不知道是谁」，同投影）、时刻落在给的范围里的（投影丢掉的那一段：导进来的更早的
//! 记录、投影从来没见过的不算）。找不到的（库被清理过、没记上）是空的。

use rusqlite::{Connection, OptionalExtension, params};

/// 场所 `venue` 里平台编号是 `msg`、时刻（毫秒）在 `from` 到 `to` 之间的那条人说的话是谁发的。
pub(crate) fn sender_of(
    connection: &Connection,
    venue: &str,
    msg: &str,
    (from, to): (i64, i64),
) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT sender FROM messages WHERE venue = ?1 AND msg = ?2 AND mine = 0 AND at BETWEEN ?3 AND ?4",
            params![venue, msg, from, to],
            |row| row.get(0),
        )
        .optional()
}

#[cfg(test)]
mod tests;
