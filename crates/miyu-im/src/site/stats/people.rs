//! 统计页的「按人」（设计 11 第三节第 3 条，施工 I-169）：这一段里每个人用了多少 token，照 im 自己的个人账本（`crate::spent`，同使用
//! 限制页），从多到少最多 [`MOST`] 个，名字照聊天记录里他最近一次说话时的。个人账本只记平台上的人、不分机器人：选了连接的只留那个
//! 平台的人；一张会话卡（浮窗的统计）不给。读不出的是空的。

use serde_json::{Value, json};

use super::super::Site;
use super::super::scope::Scope;
use crate::{records, spent, status_file};

/// 列几个人。
const MOST: usize = 100;

/// 从 `from`（毫秒）起每个人用了多少：`[{id, name, tokens}]`。
pub(super) async fn people(site: &Site, from: i64, scope: Option<&Scope>) -> Value {
    let platform = match scope {
        None => None,
        Some(Scope::Connection { platform, .. }) => Some(format!("{platform}:")),
        Some(Scope::Venues(_)) => return Value::Array(Vec::new()),
    };
    let state = status_file::path(&site.root);
    let (used, chat) = (
        state.with_file_name(spent::FILE),
        state.with_file_name(records::FILE),
    );
    let read = tokio::task::spawn_blocking(move || {
        let mut top = spent::ranking(&used, from, MOST)?;
        if let Some(platform) = &platform {
            top.retain(|(id, _)| id.starts_with(platform.as_str()));
        }
        let ids: Vec<String> = top.iter().map(|(id, _)| id.clone()).collect();
        // 聊天记录读不出的照没有名字。
        let names = records::connection(&chat)
            .and_then(|connection| records::read::names(&connection, &ids))
            .unwrap_or_default();
        Ok::<_, rusqlite::Error>((top, names))
    })
    .await;
    let Ok(Ok((top, names))) = read else {
        return Value::Array(Vec::new());
    };
    top.into_iter()
        .map(|(id, tokens)| json!({"id": id, "name": names.get(&id), "tokens": tokens}))
        .collect()
}
