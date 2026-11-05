//! 每人的用量（设计 08 第二节第 2 条，施工 I-134）：核心的用量账本没有「人」这一项，im 照会话推来的 `model.called` 自己记。
//!
//! - 一次请求的四项用量（读缓存、没命中、写缓存、输出）加起来（[`personal`]：压缩的摘要请求、辅助请求不算到人头上），平分给这一轮回的几个人（[`shares`]），余数给排在
//!   前面的。没有触发人的轮次（看一眼、终端叫她发 QQ）不记。
//! - 存：im 状态目录里一个 SQLite 库（[`FILE`]），一行一笔：会话、事件序号、人、场所、时刻（毫秒）、份额。照（会话、序号、人）
//!   去重：桥起来时照日志从头补，同一条不记两次。
//! - 只留最近 [`KEEP_MS`]：最长的一条线是 30 天，多留一天。更早的不记，记的时候顺手每小时清一次。
//! - 写在调的一头当场写（WAL、`synchronous = NORMAL`，一行只是一次不落盘的提交）：进站链判额度要马上读得到。打不开的记一行运行
//!   日志，交回一份什么都不记的。

use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI64, Ordering};

use miyu_client::events::{Body, Event};
use miyu_client::ids::ExternalId;
use rusqlite::{Connection, params};

use crate::TARGET;

#[cfg(test)]
mod tests;

/// 库文件的名字（im 状态目录里）。
pub(crate) const FILE: &str = "usage.db";

/// 留多久：30 天加一天，毫秒。
pub(crate) const KEEP_MS: i64 = 31 * DAY_MS;

/// 一天，毫秒。
const DAY_MS: i64 = 24 * HOUR_MS;

/// 一小时，毫秒：隔这么久清一次。
const HOUR_MS: i64 = 60 * 60 * 1000;

/// 建表。
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS spent (
    session TEXT NOT NULL,
    seq INTEGER NOT NULL,
    person TEXT NOT NULL,
    venue TEXT NOT NULL,
    at INTEGER NOT NULL,
    tokens INTEGER NOT NULL,
    PRIMARY KEY (session, seq, person)
);
CREATE INDEX IF NOT EXISTS spent_person_at ON spent (person, at);
";

/// 一次请求：记在哪个会话的哪一条、哪个场所、什么时刻、四项合计。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Charge<'a> {
    /// 那条线的会话编号（主线、支线、私聊）。
    pub(crate) session: &'a str,
    /// `model.called` 在那条线里的序号。
    pub(crate) seq: u64,
    /// 场所编号（`qq:group:1`）。
    pub(crate) venue: &'a str,
    /// 记下的时刻，毫秒。
    pub(crate) at: i64,
    /// 四项合计。
    pub(crate) tokens: u64,
}

/// 每人的用量账本。
pub(crate) struct Spent {
    /// 库；打不开的是空的。
    connection: Option<Mutex<Connection>>,
    /// 上一次清是什么时刻，毫秒。
    pruned: AtomicI64,
}

impl Spent {
    /// 打开 `path` 的库（没有的建上）。打不开的记一行运行日志，交回一份什么都不记的。
    pub(crate) fn open(path: &Path) -> Spent {
        let connection = match prepared(path) {
            Ok(connection) => Some(Mutex::new(connection)),
            Err(error) => {
                tracing::warn!(target: TARGET, path = %path.display(), error = %error, "usage ledger not opened");
                None
            }
        };
        Spent {
            connection,
            pruned: AtomicI64::new(0),
        }
    }

    /// 记一次请求 `charge`，平分给 `payers`；此刻是 `now`（毫秒）。没人付的、四项都是 0 的、早于 [`KEEP_MS`] 的不记。
    pub(crate) fn charge(&self, charge: &Charge<'_>, payers: &[ExternalId], now: i64) {
        let Some(connection) = &self.connection else {
            return;
        };
        if payers.is_empty() || charge.tokens == 0 || charge.at < now - KEEP_MS {
            return;
        }
        let connection = connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let shares = shares(charge.tokens, payers.len());
        let written = (|| {
            let transaction = connection.unchecked_transaction()?;
            for (payer, share) in payers.iter().zip(shares) {
                transaction.execute(
                    "INSERT OR IGNORE INTO spent (session, seq, person, venue, at, tokens) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        charge.session,
                        i64::try_from(charge.seq).unwrap_or(i64::MAX),
                        payer.as_str(),
                        charge.venue,
                        charge.at,
                        i64::try_from(share).unwrap_or(i64::MAX)
                    ],
                )?;
            }
            transaction.commit()
        })();
        if let Err(error) = written {
            tracing::warn!(target: TARGET, error = %error, "usage not written");
        }
        if now - self.pruned.load(Ordering::Relaxed) >= HOUR_MS {
            self.pruned.store(now, Ordering::Relaxed);
            if let Err(error) =
                connection.execute("DELETE FROM spent WHERE at < ?1", [now - KEEP_MS])
            {
                tracing::warn!(target: TARGET, error = %error, "usage not pruned");
            }
        }
    }
}

/// 打开 `path`、建好表。
fn prepared(path: &Path) -> rusqlite::Result<Connection> {
    if let Some(dir) = path.parent()
        && let Err(error) = std::fs::create_dir_all(dir)
    {
        tracing::debug!(target: TARGET, error = %error, "usage dir not made");
    }
    let connection = Connection::open(path)?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.execute_batch(SCHEMA)?;
    Ok(connection)
}

/// 一条事件要是一次报了用量的主请求（`model.called`），交回四项用量加起来；压缩的摘要请求、辅助请求（设计 08 第一节第 4 条：
/// 只算进会话的合计）、别的事件是空的。
pub(crate) fn personal(event: &Event) -> Option<u64> {
    let Body::ModelCalled(called) = &event.body else {
        return None;
    };
    if called.compaction.is_some() || called.purpose.is_some() {
        return None;
    }
    let usage = called.usage.as_ref()?;
    Some(
        usage
            .uncached
            .saturating_add(usage.cache_read)
            .saturating_add(usage.cache_write)
            .saturating_add(usage.output),
    )
}

/// `tokens` 平分成 `payers` 份，余数一份一个给排在前面的。
pub(crate) fn shares(tokens: u64, payers: usize) -> Vec<u64> {
    let Ok(count) = u64::try_from(payers) else {
        return Vec::new();
    };
    if count == 0 {
        return Vec::new();
    }
    let (each, rest) = (tokens / count, tokens % count);
    (0..count)
        .map(|index| each + u64::from(index < rest))
        .collect()
}
