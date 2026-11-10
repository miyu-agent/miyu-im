//! 每人的用量（设计 08 第二节第 2 条，施工 I-134）：核心的用量账本没有「人」这一项，im 照会话推来的 `model.called` 自己记。
//!
//! - 一次请求的四项用量（读缓存、没命中、写缓存、输出）加起来（[`personal`]：压缩的摘要请求、辅助请求不算到人头上），平分给这一轮回的几个人（[`shares`]），余数给排在
//!   前面的。没有触发人的轮次（看一眼、终端叫她发 QQ）不记。
//! - 金额（施工 I-205，项目主人 11-10：使用限制页要看得到每人已花的钱）：`model.called` 带着核心算好的这一次的金额（`cost`，原
//!   币种、不取整），照同样的办法平分，连币种一起记；核心算不出价的（没有 `cost`）、施工 I-205 以前记的那些行金额是空的，读的
//!   时候算成「没记录金额的请求」。换成显示的币种是核心的事（`usage.convert`），这里只照「人、日子、币种」加好交出去。
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
#[derive(Debug, Clone, PartialEq)]
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
    /// 核心算的这一次的金额和币种（`model.called` 的 `cost`）；算不出价的是空的。
    pub(crate) cost: Option<(f64, &'a str)>,
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
        // 金额照人数平分（不取整，核心给的就是小数）。
        let each = charge.cost.map(|(amount, _)| amount / payers.len() as f64);
        let currency = charge.cost.map(|(_, currency)| currency);
        let written = (|| {
            let transaction = connection.unchecked_transaction()?;
            for (payer, share) in payers.iter().zip(shares) {
                transaction.execute(
                    "INSERT OR IGNORE INTO spent (session, seq, person, venue, at, tokens, amount, currency) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        charge.session,
                        i64::try_from(charge.seq).unwrap_or(i64::MAX),
                        payer.as_str(),
                        charge.venue,
                        charge.at,
                        i64::try_from(share).unwrap_or(i64::MAX),
                        each,
                        currency
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

    /// `person` 从 `from`（毫秒，含）起用了多少。读不出的是 0。
    pub(crate) fn since(&self, person: &str, from: i64) -> u64 {
        let Some(connection) = &self.connection else {
            return 0;
        };
        let connection = connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let summed = connection.query_row(
            "SELECT COALESCE(SUM(tokens), 0) FROM spent WHERE person = ?1 AND at >= ?2",
            params![person, from],
            |row| row.get::<_, i64>(0),
        );
        match summed {
            Ok(sum) => u64::try_from(sum).unwrap_or_default(),
            Err(error) => {
                tracing::warn!(target: TARGET, error = %error, "usage not read");
                0
            }
        }
    }
}

/// 页面读（使用限制页，施工 I-136）：库 `path` 里最近 30 天用过的人，各自最近 5 小时、7 天、30 天用了多少（此刻是 `now`，
/// 毫秒），照 5 小时的用量从多到少排，再照 30 天的，最多 `most` 个。只读地开：还没有库的是空的，不建。
pub(crate) fn top(path: &Path, now: i64, most: usize) -> rusqlite::Result<Vec<(String, [u64; 3])>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    let [five, week, month] = crate::limits::WINDOWS.map(|window| now - window);
    let mut statement = connection.prepare(
        "SELECT person,
                SUM(CASE WHEN at >= ?1 THEN tokens ELSE 0 END),
                SUM(CASE WHEN at >= ?2 THEN tokens ELSE 0 END),
                SUM(tokens)
         FROM spent WHERE at >= ?3 GROUP BY person ORDER BY 2 DESC, 4 DESC, person LIMIT ?4",
    )?;
    let unsigned = |sum: i64| u64::try_from(sum).unwrap_or_default();
    let rows = statement.query_map(
        params![five, week, month, i64::try_from(most).unwrap_or(i64::MAX)],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                [
                    unsigned(row.get(1)?),
                    unsigned(row.get(2)?),
                    unsigned(row.get(3)?),
                ],
            ))
        },
    )?;
    rows.collect()
}

/// 页面读（数据统计页的「按人」，施工 I-169）：库 `path` 里 `from`（毫秒）以后用过的人，各用了多少，从多到少最多 `most` 个。只读地
/// 开：还没有库的是空的，不建。
pub(crate) fn ranking(path: &Path, from: i64, most: usize) -> rusqlite::Result<Vec<(String, u64)>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    let mut statement = connection.prepare(
        "SELECT person, SUM(tokens) FROM spent WHERE at >= ?1 GROUP BY person ORDER BY 2 DESC, person LIMIT ?2",
    )?;
    let rows = statement.query_map(
        params![from, i64::try_from(most).unwrap_or(i64::MAX)],
        |row| {
            let sum: i64 = row.get(1)?;
            Ok((
                row.get::<_, String>(0)?,
                u64::try_from(sum).unwrap_or_default(),
            ))
        },
    )?;
    rows.collect()
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
    // 施工 I-205 以前建的库没有金额的两列：补上（老的行是空的，照「没记录金额」读）。
    let has_amount = connection
        .prepare("SELECT 1 FROM pragma_table_info('spent') WHERE name = 'amount'")?
        .exists([])?;
    if !has_amount {
        connection.execute_batch(
            "ALTER TABLE spent ADD COLUMN amount REAL; ALTER TABLE spent ADD COLUMN currency TEXT;",
        )?;
    }
    Ok(connection)
}

/// 一段时间里的金额（施工 I-205）：照「日子（UTC，`2026-11-10`）、币种」加好的几笔，和没记录金额的请求有几次。
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Costs {
    /// （日子、币种、金额），照日子、币种排。
    pub(crate) parts: Vec<(String, String, f64)>,
    /// 有用量、没有金额的请求（核心算不出价的、I-205 以前记的）。
    pub(crate) unpriced: u64,
}

/// 页面读（使用限制页的金额，施工 I-205）：库 `path` 里 `people` 这几个人各自最近 5 小时、7 天、30 天的金额（此刻是 `now`，
/// 毫秒）。只读地开：还没有库、还没有金额两列（桥没起来过）的是空的。
pub(crate) fn costs(
    path: &Path,
    people: &[String],
    now: i64,
) -> rusqlite::Result<std::collections::HashMap<String, [Costs; 3]>> {
    let mut found = std::collections::HashMap::new();
    if !path.exists() || people.is_empty() {
        return Ok(found);
    }
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    let windows = crate::limits::WINDOWS.map(|window| now - window);
    let Ok(mut statement) =
        connection.prepare("SELECT at, amount, currency FROM spent WHERE person = ?1 AND at >= ?2")
    else {
        return Ok(found);
    };
    for person in people {
        let mut sums: [std::collections::BTreeMap<(String, String), f64>; 3] = Default::default();
        let mut unpriced = [0u64; 3];
        let rows = statement.query_map(params![person, windows[2]], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<f64>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        for row in rows {
            let (at, amount, currency) = row?;
            for (index, from) in windows.iter().enumerate() {
                if at < *from {
                    continue;
                }
                match (amount, &currency) {
                    (Some(amount), Some(currency)) => {
                        *sums[index].entry((day(at), currency.clone())).or_default() += amount;
                    }
                    _ => unpriced[index] += 1,
                }
            }
        }
        let mut made = sums
            .into_iter()
            .zip(unpriced)
            .map(|(sums, unpriced)| Costs {
                parts: sums
                    .into_iter()
                    .map(|((day, currency), amount)| (day, currency, amount))
                    .collect(),
                unpriced,
            });
        let three = std::array::from_fn(|_| made.next().unwrap_or_default());
        found.insert(person.clone(), three);
    }
    Ok(found)
}

/// 时刻 `at`（毫秒）是 UTC 的哪一天：`2026-11-10`（核心换汇率照 UTC 的日子）。
fn day(at: i64) -> String {
    let stamp = miyu_client::time::Timestamp::from_unix_millis(at).map(|at| at.to_string());
    stamp
        .and_then(|text| text.get(..10).map(str::to_string))
        .unwrap_or_default()
}

/// 一条报了用量的主请求（同 [`personal`]）带的金额和币种（施工 I-205）；核心算不出价的是空的。
pub(crate) fn priced(event: &Event) -> Option<(f64, String)> {
    let Body::ModelCalled(called) = &event.body else {
        return None;
    };
    let cost = called.cost.as_ref()?;
    Some((cost.amount.get(), cost.currency.clone()))
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
