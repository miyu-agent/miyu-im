//! 聊天记录（设计 04，施工 C-1 上；2026-10-23 项目主人定：聊天记录是通讯平台自己的数据，和核心没关系，im 自己存一份）：各平台
//! 通用，照平台身份、场所编号区分。AI 的会话（核心的）、回复判断日志（会话事件、运行日志）各管各的，这里只记聊天。
//!
//! - 存：im 状态目录里一个 SQLite 库（[`FILE`]）。一张消息表（`messages`：场所、平台编号、谁、名字、时刻、正文、引用、@ 了谁、
//!   带的东西、是不是她发的、撤回），正文另建全文索引（FTS5，`trigram` 分词：中文不用分词库，三个字以上能搜子串）；一张事件表
//!   （`happenings`：禁言、解禁、踢人、进群、退群、撤回）。
//! - 带的东西只记占位和平台编号（种类、编号、名字、大小），不存内容：要看的时候拿编号去取，取不到的说可能已经失效。
//! - 写：一个专门的线程排着写（[`Records::write`] 只是交过去，跟核心的那一头不等它）；同一个场所、同一个平台编号的消息只记一次
//!   （断线重发、补拉）。写不进的记一行运行日志，照跑。
//! - 判断（施工 C-1 中）：一条消息的回没回、为什么另记一张表 `judgments`，消息记录页照它画；她查聊天记录的工具不读它。
//! - 读、删（施工 C-1 中，`read.rs`、`delete.rs`）：页面服务另开连接（WAL，读写不互相挡；删的时候等写的线程让出锁）。
//! - 她查（施工 C-1 下，`history.rs`）：驱动的工具经 [`Records::history`] 只读地搜，判断不读。

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use rusqlite::{Connection, params};
use serde_json::Value;

use crate::TARGET;

pub(crate) mod delete;
pub(crate) mod history;
pub(crate) mod read;

/// 库文件的名字（im 状态目录里）。
pub(crate) const FILE: &str = "records.db";

/// 库的格式版本（`user_version`）：改表的时候往上加，照它迁移。
const VERSION: i64 = 1;

/// 建表。
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY,
    venue TEXT NOT NULL,
    msg TEXT NOT NULL,
    sender TEXT NOT NULL,
    name TEXT,
    at INTEGER NOT NULL,
    text TEXT NOT NULL,
    reply_to TEXT,
    mentions TEXT NOT NULL DEFAULT '[]',
    media TEXT NOT NULL DEFAULT '[]',
    mine INTEGER NOT NULL DEFAULT 0,
    recalled_at INTEGER,
    recalled_by TEXT,
    UNIQUE (venue, msg)
);
CREATE INDEX IF NOT EXISTS messages_venue_at ON messages (venue, at);
CREATE INDEX IF NOT EXISTS messages_sender_at ON messages (sender, at);
CREATE VIRTUAL TABLE IF NOT EXISTS messages_text USING fts5 (
    text, content = 'messages', content_rowid = 'id', tokenize = 'trigram'
);
CREATE TRIGGER IF NOT EXISTS messages_text_insert AFTER INSERT ON messages BEGIN
    INSERT INTO messages_text (rowid, text) VALUES (new.id, new.text);
END;
CREATE TRIGGER IF NOT EXISTS messages_text_delete AFTER DELETE ON messages BEGIN
    INSERT INTO messages_text (messages_text, rowid, text) VALUES ('delete', old.id, old.text);
END;
CREATE TABLE IF NOT EXISTS happenings (
    id INTEGER PRIMARY KEY,
    venue TEXT NOT NULL,
    at INTEGER NOT NULL,
    kind TEXT NOT NULL,
    target TEXT,
    actor TEXT,
    seconds INTEGER,
    msg TEXT
);
CREATE INDEX IF NOT EXISTS happenings_venue_at ON happenings (venue, at);
CREATE TABLE IF NOT EXISTS judgments (
    venue TEXT NOT NULL,
    msg TEXT NOT NULL,
    at INTEGER NOT NULL,
    outcome TEXT NOT NULL,
    why TEXT NOT NULL,
    PRIMARY KEY (venue, msg)
);
";

/// 一条消息（人说的、她说的）。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Said {
    /// 场所编号（`qq:group:123`）。
    pub(crate) venue: String,
    /// 平台给这条消息的编号。
    pub(crate) msg: String,
    /// 谁：平台身份（`qq:10001`）。
    pub(crate) sender: String,
    /// 当时的名字（群名片、昵称），没有的是空的。
    pub(crate) name: Option<String>,
    /// 什么时候：毫秒的 Unix 时间。
    pub(crate) at: i64,
    /// 正文：@ 写成 `@名字`，合并转发、卡片写占位。
    pub(crate) text: String,
    /// 引用的那一条的平台编号。
    pub(crate) reply_to: Option<String>,
    /// @ 了谁：平台身份。
    pub(crate) mentions: Vec<String>,
    /// 带的东西：`{kind, id, name?, size?}`，不存内容。
    pub(crate) media: Vec<Value>,
    /// 是不是她发的。
    pub(crate) mine: bool,
}

/// 群里的一件事（禁言、解禁、踢人、进群、退群、撤回）：谁做的、对谁。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Happening {
    /// 场所编号。
    pub(crate) venue: String,
    /// 什么时候：毫秒的 Unix 时间。
    pub(crate) at: i64,
    /// 哪一种：`muted`、`unmuted`、`kicked`、`joined`、`left`、`recalled`。
    pub(crate) kind: &'static str,
    /// 对谁：平台身份；不知道的是空的。
    pub(crate) target: Option<String>,
    /// 谁做的：平台身份；不知道的是空的。
    pub(crate) actor: Option<String>,
    /// 禁言多少秒。
    pub(crate) seconds: Option<i64>,
    /// 撤回的那一条的平台编号。
    pub(crate) msg: Option<String>,
}

/// 要写的一件。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Write {
    /// 一条消息：同一个场所、同一个编号的只记第一次。
    Said(Box<Said>),
    /// 一件事；撤回的另把那一条标成撤回了（什么时候、谁撤的）。
    Happened(Happening),
    /// 一条消息的判断（施工 C-1 中，`judgments` 表）：同一条判过几次的照后一次。消息记录页照它在那一条底下写回没回、为什么；她查
    /// 聊天记录的工具不读这张表（项目主人 10-23：判断日志不让她查到）。
    Judged {
        /// 场所编号。
        venue: String,
        /// 那一条的平台编号。
        msg: String,
        /// 结论：`reply`、`record`、`deferred`。
        outcome: String,
        /// 为什么（`noted` 说的，同会话浮窗的最近判断）。
        why: Value,
    },
}

/// 聊天记录：交写的（读的随消息记录页，施工 C-1 中）。
pub(crate) struct Records {
    /// 交给写的线程；库打不开的是空的（照跑，不记）。
    writes: Option<mpsc::Sender<Write>>,
    /// 库文件。
    path: PathBuf,
}

impl Records {
    /// 打开 `path` 的库（没有的建上），起写的线程。打不开的记一行运行日志，交回一份什么都不记的。
    pub(crate) fn open(path: PathBuf) -> Records {
        let connection = match prepared(&path) {
            Ok(connection) => connection,
            Err(error) => {
                tracing::warn!(target: TARGET, path = %path.display(), error = %error, "records not opened");
                return Records { writes: None, path };
            }
        };
        let (sender, received) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("im-records".to_string())
            .spawn(move || writer(connection, &received));
        if let Err(error) = spawned {
            tracing::warn!(target: TARGET, error = %error, "records writer not started");
            return Records { writes: None, path };
        }
        Records {
            writes: Some(sender),
            path,
        }
    }

    /// 只读地搜的口子（她查聊天记录的工具用，施工 C-1 下）。
    pub(crate) fn history(&self) -> history::History {
        history::History::new(self.path.clone())
    }

    /// 交一件去写，不等。
    pub(crate) fn write(&self, write: Write) {
        if let Some(writes) = &self.writes {
            // 写的线程没了（照说不会）：这一件不记。
            if writes.send(write).is_err() {
                tracing::debug!(target: TARGET, "records writer gone");
            }
        }
    }
}

/// 打开 `path`、建好表、迁到 [`VERSION`]。
fn prepared(path: &Path) -> rusqlite::Result<Connection> {
    if let Some(dir) = path.parent()
        && let Err(error) = std::fs::create_dir_all(dir)
    {
        tracing::debug!(target: TARGET, error = %error, "records dir not made");
    }
    let connection = Connection::open(path)?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.execute_batch(SCHEMA)?;
    connection.pragma_update(None, "user_version", VERSION)?;
    Ok(connection)
}

/// 写的线程：排着写，交的一头没了就停。
fn writer(connection: Connection, received: &mpsc::Receiver<Write>) {
    while let Ok(write) = received.recv() {
        if let Err(error) = written(&connection, &write) {
            tracing::warn!(target: TARGET, error = %error, "record not written");
        }
    }
}

/// 写一件。
pub(crate) fn written(connection: &Connection, write: &Write) -> rusqlite::Result<()> {
    match write {
        Write::Said(said) => {
            let mentions = serde_json::to_string(&said.mentions).unwrap_or_else(|_| "[]".into());
            let media = serde_json::to_string(&said.media).unwrap_or_else(|_| "[]".into());
            connection.execute(
                "INSERT OR IGNORE INTO messages (venue, msg, sender, name, at, text, reply_to, mentions, media, mine)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    said.venue,
                    said.msg,
                    said.sender,
                    said.name,
                    said.at,
                    said.text,
                    said.reply_to,
                    mentions,
                    media,
                    said.mine
                ],
            )?;
        }
        Write::Happened(happening) => {
            connection.execute(
                "INSERT INTO happenings (venue, at, kind, target, actor, seconds, msg) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    happening.venue,
                    happening.at,
                    happening.kind,
                    happening.target,
                    happening.actor,
                    happening.seconds,
                    happening.msg
                ],
            )?;
            if let (RECALLED, Some(msg)) = (happening.kind, &happening.msg) {
                connection.execute(
                    "UPDATE messages SET recalled_at = ?1, recalled_by = ?2 WHERE venue = ?3 AND msg = ?4",
                    params![happening.at, happening.actor, happening.venue, msg],
                )?;
            }
        }
        Write::Judged {
            venue,
            msg,
            outcome,
            why,
        } => {
            connection.execute(
                "INSERT OR REPLACE INTO judgments (venue, msg, at, outcome, why) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![venue, msg, now(), outcome, why.to_string()],
            )?;
        }
    }
    Ok(())
}

/// 页面服务读、删用的连接（施工 C-1 中）：打开 `path` 的库（没有的建上表），写的线程占着锁时最多等几秒。
///
/// # Errors
///
/// 打不开。
pub(crate) fn connection(path: &Path) -> rusqlite::Result<Connection> {
    let connection = prepared(path)?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(connection)
}

/// 撤回那一种事件的 `kind`。
pub(crate) const RECALLED: &str = "recalled";

/// 此刻：毫秒的 Unix 时间。
pub(crate) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
        })
}

/// 平台给的时刻 `seconds`（整数秒，没带的是 0）换成毫秒；没带的照此刻。
pub(crate) fn at(seconds: i64) -> i64 {
    if seconds > 0 {
        seconds.saturating_mul(1000)
    } else {
        now()
    }
}

#[cfg(test)]
mod tests;
