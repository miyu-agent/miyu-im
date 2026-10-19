//! 页面经通道调的方法（施工 I-6 上起）：照这一刻桥手里的样子答，不读盘。
//!
//! - `whoami` → `{"account"}`：这份登录的账号。
//! - `connections` → `{"connections": [...]}`：机器人照号排，连着的在前、断开了的（施工 I-6 中：「断开 4 分钟，在重连」）在后。
//!   每个：`platform`（驱动给的平台名）、`bot`（号，写成字）、`connected`、`since`（连上的、断开的时刻，毫秒的 Unix 时间），问到了
//!   的带 `name`（机器人在平台上叫什么，`faces`）、`implementation`、`version`；头像模板有的带 `avatar`（页面服务转的地址）。
//! - `status` → 驱动报的几格（OneBot 是 `listen`、`token`）、`platform`、`path`（NapCat 连进来的路径里最短的那个），同后台页的
//!   `status`（施工 I-6 中：新建连接、连接设置拼地址）。
//! - `connection.save {values}` → `{"saved": true}`：存驱动的配置项（`save`）。
//! - `today` → `{"received", "replied", "recorded", "undelivered"}`：今天的几个数（施工 I-6 下，`crate::counters`）。
//! - `logs {level?, limit?}` → `{"logs": [...]}`：最近的运行日志，新的在前（`logs`；`level` 是 `all`、`warn`、`error`，`limit`
//!   不写的 50）。
//! - `cards.get`、`cards.save_one`、`cards.add`、`cards.remove`：会话页的（施工 I-7 上），经人的连接转给读的一头（`venues`）。
//! - `models`：会话浮窗选模型（施工 I-7 中一，`venues::models`）。
//! - `reactions` → `{"reactions": [{id, name, glyph?}], "default"}`：能贴的表情（驱动给的，空的是这个平台不支持贴表情）和桥出厂
//!   贴的那一个（`bridge.json` 的 `reaction_emoji`，施工 I-7 中三下二）。
//! - 别的：`unknown_method`。

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::logins::Login;
use super::{Person, Site, logs, save, venues};
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
        Some("today") => {
            let day = site.counters.today();
            Ok(
                json!({"received": day.received, "replied": day.replied, "recorded": day.recorded, "undelivered": day.undelivered}),
            )
        }
        Some("logs") => {
            let file = site.root.state().join("logs").join("im.log");
            let filter = params["level"].as_str().unwrap_or("all");
            let limit = params["limit"]
                .as_u64()
                .map_or(50, |limit| usize::try_from(limit).unwrap_or(logs::MOST));
            Ok(json!({"logs": logs::recent(&file, filter, limit)}))
        }
        Some("models") => venues::models(&login.person).await,
        Some("reactions") => Ok(reactions(site)),
        Some(method) if venues::FORWARDED.contains(&method) => {
            venues::forward(&login.person, method, params).await
        }
        _ => Err(("unknown_method".to_string(), String::new())),
    }
}

/// 能贴的表情（模块的说明）。
fn reactions(site: &Arc<Site>) -> Value {
    let reactions: Vec<Value> = site
        .driver
        .reactions()
        .iter()
        .map(|one| {
            let mut shown = json!({"id": one.id, "name": one.name});
            if let Some(glyph) = one.glyph {
                shown["glyph"] = json!(glyph);
            }
            shown
        })
        .collect();
    json!({"reactions": reactions, "default": site.running.tuning.reaction_emoji})
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
