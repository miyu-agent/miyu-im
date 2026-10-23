//! 消息记录页要的方法（施工 C-1 中，设计 04 第一节第 2、3 条）：读、搜、删 im 自己的聊天记录库（`crate::records`）。库在 im 自己
//! 的状态目录里，页面服务就在 im 的进程里，直接开连接读写，不经核心。读写的事放到阻塞的线程里做。
//!
//! - `records.venues` → `{venues: [{venue, last, count}]}`：有记录的场所（`records::read::venues`），最近说过话的在前。
//! - `records.page {venue, before?}` → `{items, more, before}`：一页（`records::read::page`），从旧到新，带判断和群事件。
//! - `records.search {query?, venue?, sender?, from?, to?}` → `{items}`：新的在前，最多 200 条。
//! - `records.delete {venue?, venues?, from?, to?, sender?, all?}` → `{deleted}`：删了几条消息（`venues` 一次删几个场所的）；什么都没写、又没写 `all` 的不删。
//!
//! 参数写错的拒 `bad_params`；库打不开、读写不成的拒 `records_failed`（原话交回）。

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};

use super::Site;
use crate::records::{self, delete, read};
use crate::status_file;

/// 一页几条。
const PAGE: i64 = 60;

/// 搜一次最多几条。
const FOUND: i64 = 200;

/// 拒的原因和原话。
type Refused = (String, String);

/// 库文件。
fn path(site: &Site) -> PathBuf {
    status_file::path(&site.root).with_file_name(records::FILE)
}

/// 参数写错的。
fn bad() -> Refused {
    ("bad_params".to_string(), String::new())
}

/// 在阻塞的线程里开库办 `work`。
async fn with<T: Send + 'static>(
    site: &Arc<Site>,
    work: impl FnOnce(&mut rusqlite::Connection) -> rusqlite::Result<T> + Send + 'static,
) -> Result<T, Refused> {
    let path = path(site);
    let done = tokio::task::spawn_blocking(move || {
        let mut connection = records::connection(&path)?;
        work(&mut connection)
    })
    .await
    .map_err(|error| ("records_failed".to_string(), error.to_string()))?;
    done.map_err(|error| ("records_failed".to_string(), error.to_string()))
}

/// 毫秒的 Unix 时间（非负整数）；没写的是空的，写错的是 `Err`。
fn millis(value: &Value) -> Result<Option<i64>, Refused> {
    match value {
        Value::Null => Ok(None),
        value => value
            .as_i64()
            .filter(|at| *at >= 0)
            .map(Some)
            .ok_or_else(bad),
    }
}

/// 不空的字；没写的是空的，不是字的是 `Err`。
fn text(value: &Value) -> Result<Option<String>, Refused> {
    match value {
        Value::Null => Ok(None),
        Value::String(text) => Ok(Some(text.trim().to_string()).filter(|text| !text.is_empty())),
        _ => Err(bad()),
    }
}

/// 场所：`venue` 一个、`venues` 几个，两样合起来；都没写的是空的，不是字的是 `Err`。
fn venues(params: &Value) -> Result<Vec<String>, Refused> {
    let mut venues: Vec<String> = text(&params["venue"])?.into_iter().collect();
    match &params["venues"] {
        Value::Null => {}
        Value::Array(list) => {
            for one in list {
                venues.extend(text(one)?);
            }
        }
        _ => return Err(bad()),
    }
    Ok(venues)
}

/// `records.venues`（模块的说明）。
pub(super) async fn listed(site: &Arc<Site>) -> Result<Value, Refused> {
    with(site, |connection| read::venues(connection)).await
}

/// `records.page`（模块的说明）。
pub(super) async fn page(site: &Arc<Site>, params: &Value) -> Result<Value, Refused> {
    let venue = text(&params["venue"])?.ok_or_else(bad)?;
    let before = millis(&params["before"])?;
    with(site, move |connection| {
        read::page(connection, &venue, before, PAGE)
    })
    .await
}

/// `records.search`（模块的说明）。
pub(super) async fn search(site: &Arc<Site>, params: &Value) -> Result<Value, Refused> {
    let query = read::Query {
        text: text(&params["query"])?.unwrap_or_default(),
        venue: text(&params["venue"])?,
        sender: text(&params["sender"])?,
        from: millis(&params["from"])?,
        to: millis(&params["to"])?,
        limit: FOUND,
    };
    with(site, move |connection| read::search(connection, &query)).await
}

/// `records.delete`（模块的说明）。
pub(super) async fn remove(site: &Arc<Site>, params: &Value) -> Result<Value, Refused> {
    let scope = delete::Scope {
        venues: venues(params)?,
        from: millis(&params["from"])?,
        to: millis(&params["to"])?,
        sender: text(&params["sender"])?,
        all: params["all"].as_bool().unwrap_or(false),
    };
    let deleted = with(site, move |connection| delete::delete(connection, &scope)).await?;
    tracing::info!(target: crate::TARGET, deleted, "records deleted");
    Ok(json!({"deleted": deleted}))
}
