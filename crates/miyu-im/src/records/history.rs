//! 她查聊天记录的口子（施工 C-1 下，设计 04 第一节第 4 条）：驱动的工具经它只读 im 的聊天记录库搜消息（`read::search`），在阻塞的
//! 线程里开一个连接读完就关。回复判断那张表不读（项目主人 10-23：判断日志不让她查到），也不能删。

use std::path::PathBuf;

use serde_json::Value;

use super::read;

/// 搜的条件（同消息记录页的搜，`read::Query`）。
#[derive(Debug, Clone, Default)]
pub struct HistoryQuery {
    /// 关键词：空的不按字筛。
    pub text: String,
    /// 只在这个场所里搜（`qq:group:777`）：空的跨场所。
    pub venue: Option<String>,
    /// 谁：全是数字的照号，别的照名字的子串。
    pub sender: Option<String>,
    /// 从（毫秒的 Unix 时间，含）。
    pub from: Option<i64>,
    /// 到（毫秒的 Unix 时间，不含）。
    pub to: Option<i64>,
    /// 最多几条（新的那几条）。
    pub limit: i64,
}

/// 聊天记录库，只读。
#[derive(Debug, Clone)]
pub struct History {
    path: PathBuf,
}

impl History {
    /// 库在 `path`。
    pub(crate) fn new(path: PathBuf) -> History {
        History { path }
    }

    /// 照 `query` 搜：新的在前，一条一个对象（同 `records.search` 的一项：`venue`、`msg`、`sender`、`name`、`at`、`text`、
    /// `reply_to`、`media`、`mine`、`recalled_at`……）。
    ///
    /// # Errors
    ///
    /// 库打不开、读不出：原话。
    pub async fn search(&self, query: HistoryQuery) -> Result<Vec<Value>, String> {
        let path = self.path.clone();
        let found = tokio::task::spawn_blocking(move || {
            let connection = super::connection(&path)?;
            let query = read::Query {
                text: query.text,
                venue: query.venue,
                sender: query.sender,
                from: query.from,
                to: query.to,
                limit: query.limit,
            };
            read::search(&connection, &query)
        })
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
        Ok(found["items"].as_array().cloned().unwrap_or_default())
    }
}
