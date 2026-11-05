//! 使用限制页（设计 08 第二节第 6 条，施工 I-136）：每人的三条线。
//!
//! - `limits.get` → `{default, people, usage}`：im 配置目录里的 `limits.toml`（默认的三条线、单独设的人），和最近 30 天用过的人
//!   （照 5 小时的用量从多到少，最多 [`MOST`] 个）：`{id, name, used, caps, full}`，`used`、`caps` 照 5 小时、7 天、30 天排，没写的线
//!   `null`；`name` 照聊天记录里他最近一次说话时的名字，没有的 `null`。用量页面服务直接开库读（同消息记录页）。
//! - `limits.set {default, people}`：整份替换 `limits.toml`（`crate::limits::People::from_page` 查，有一处不对整份不收）。桥隔几秒
//!   看一眼文件变没变，下一条消息照新的。

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};

use super::Site;
use crate::limits::{FILE, People};
use crate::{records, spent, status_file};

/// 列几个人。
const MOST: usize = 50;

/// 拒的原因和原话。
type Refused = (String, String);

/// 设置文件。
fn file(site: &Site) -> PathBuf {
    crate::dirs::config(&site.root).join(FILE)
}

/// `limits.get`。
pub(super) async fn get(site: &Arc<Site>) -> Result<Value, Refused> {
    let people = People::read(&file(site));
    let state = status_file::path(&site.root);
    let (used, records) = (
        state.with_file_name(spent::FILE),
        state.with_file_name(records::FILE),
    );
    let read = tokio::task::spawn_blocking(move || {
        let top = spent::top(&used, records::now(), MOST)?;
        let ids: Vec<String> = top.iter().map(|(id, _)| id.clone()).collect();
        // 聊天记录读不出的照没有名字。
        let names = records::connection(&records)
            .and_then(|connection| records::read::names(&connection, &ids))
            .unwrap_or_default();
        Ok::<_, rusqlite::Error>((top, names))
    })
    .await
    .map_err(|error| ("limits_failed".to_string(), error.to_string()))?;
    let (top, names) = read.map_err(|error| ("limits_failed".to_string(), error.to_string()))?;
    let usage: Vec<Value> = top
        .into_iter()
        .map(|(id, used)| {
            let lines = people.lines_for(&id);
            json!({
                "id": id,
                "name": names.get(&id),
                "used": used,
                "caps": lines.caps(),
                "full": lines.reached(used),
            })
        })
        .collect();
    let listed: Vec<Value> = people.person.iter().map(|one| json!(one)).collect();
    Ok(json!({"default": people.default, "people": listed, "usage": usage}))
}

/// `limits.set`。
pub(super) fn set(site: &Site, params: &Value) -> Result<Value, Refused> {
    let people = People::from_page(&params["default"], &params["people"])
        .ok_or_else(|| ("bad_params".to_string(), String::new()))?;
    people
        .write(&file(site))
        .map_err(|error| ("write_failed".to_string(), error.to_string()))?;
    Ok(json!({"saved": true}))
}
