//! 页面经通道调的方法（施工 I-6 上起）：照这一刻桥手里的样子答，不读盘。
//!
//! - `whoami` → `{"account"}`：这份登录的账号。
//! - `connections` → `{"connections": [...]}`：机器人照号排，连着的在前、断开了的（施工 I-6 中：「断开 4 分钟，在重连」）在后。
//!   每个：`platform`（驱动给的平台名）、`bot`（号，写成字）、`connected`、`since`（连上的、断开的时刻，毫秒的 Unix 时间），问到了
//!   的带 `name`（机器人在平台上叫什么，`faces`）、`implementation`、`version`；头像模板有的带 `avatar`（页面服务转的地址）。
//! - `status` → 驱动报的几格（OneBot 是 `listen`、`token`）、`platform`、`path`（NapCat 连进来的路径里最短的那个），同后台页的
//!   `status`（施工 I-6 中：新建连接、连接设置拼地址）。
//! - `connection.save {values}` → `{"saved": true}`：存驱动的配置项（`save`）。
//! - 别的：`unknown_method`。

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::logins::Login;
use super::{Person, Site, save};
use crate::core::methods::short_path;

/// 答方法 `method`（参数 `params`）：交回 `result`，不成的交回（原因、原话，原话可以空）。
pub(super) async fn answer(
    site: &Arc<Site>,
    login: &Login<Person>,
    (method, params): (Option<&str>, &Value),
) -> Result<Value, (String, String)> {
    match method {
        Some("whoami") => Ok(json!({"account": login.account})),
        Some("connections") => Ok(json!({"connections": connections(site)})),
        Some("status") => {
            let mut status = site.running.status();
            status.insert("path".into(), json!(short_path(&site.running.tuning.paths)));
            Ok(Value::Object(status))
        }
        Some("connection.save") => save::save(site, &login.person, params).await,
        _ => Err(("unknown_method".to_string(), String::new())),
    }
}

/// 机器人（模块的说明）。
fn connections(site: &Arc<Site>) -> Vec<Value> {
    let platform = site.running.platform.name();
    let avatars = !site.running.tuning.avatar_user_url.is_empty();
    let card = |bot: i64, connected: bool, since: SystemTime| {
        let mut one = Map::new();
        one.insert("platform".into(), json!(platform));
        one.insert("bot".into(), json!(bot.to_string()));
        one.insert("connected".into(), json!(connected));
        one.insert("since".into(), json!(millis(since)));
        if let Some(name) = site.faces.name(site, bot) {
            one.insert("name".into(), json!(name));
        }
        if avatars {
            one.insert("avatar".into(), json!(format!("/avatar/{bot}")));
        }
        one
    };
    let mut all: Vec<Value> = site
        .running
        .bots
        .held()
        .into_iter()
        .map(|(bot, link, since)| {
            let mut one = card(bot, true, since);
            if let Some(implementation) = link.implementation() {
                one.insert("implementation".into(), json!(implementation.name));
                one.insert("version".into(), json!(implementation.version));
            }
            Value::Object(one)
        })
        .collect();
    all.extend(
        site.running
            .bots
            .gone()
            .into_iter()
            .map(|(bot, at)| Value::Object(card(bot, false, at))),
    );
    all
}

/// 一个时刻写成毫秒的 Unix 时间（早于 1970 年的是 0）。
fn millis(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH).map_or(0, |since| {
        u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
    })
}
