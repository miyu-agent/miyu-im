//! 页面经通道调的方法（施工 I-6 上起）：照这一刻桥手里的样子答，不读盘。
//!
//! - `connections` → `{"connections": [...]}`：挂着的每个平台的机器人（施工 I-85 上，照驱动的先后），一个平台里照号排，连着的在前、
//!   断开了的（施工 I-6 中：「断开 4 分钟，在重连」）在后。每个：`id`（连接编号 `<平台>:<号>`）、`platform`（驱动给的平台名）、
//!   `bot`（号，写成字）、`connected`、`since`（连上的、断开的时刻，毫秒的 Unix 时间）、`caps`（平台做得到的几样，施工 I-85 上），
//!   问到了的带 `name`（机器人在平台上叫什么，`faces`）、`implementation`、`version`；头像模板有的带 `avatar`（页面服务转的地址）。
//! - 下面几样多收一格 `platform`（施工 I-85 上）：不写的照第一个驱动，写了没挂的拒 `bad_params`：`status`、`connection.save`、
//!   `reactions`、`plugins.widget`。`records.media` 照场所编号的平台，驱动带的插件照带它的驱动。
//! - `stats`、`logs` 多收一格 `connection`（施工 I-85 下，首页选中的连接，`scope`）：不写的是全部连接。
//! - `status` → 驱动报的几格（OneBot 是 `listen`、`token`）、`platform`、`path`（NapCat 连进来的路径里最短的那个），同后台页的
//!   `status`（施工 I-6 中：新建连接、连接设置拼地址）。
//! - `connection.save {values}` → `{"saved": true}`：存驱动的配置项（`save`）。
//! - `today` → `{"received", "replied", "recorded", "undelivered"}`：今天的几个数（施工 I-6 下，`crate::counters`）。
//! - `logs {level?, limit?, connection?}` → `{"logs": [...]}`：最近的运行日志，新的在前（`logs`；`level` 是 `all`、`warn`、`error`，
//!   `limit` 不写的 50）。
//! - `cards.get`、`cards.save_one`、`cards.add`、`cards.remove`：会话页的（施工 I-7 上），经人的连接转给读的一头（`venues`）。
//! - `sessions.list`、`sessions.delete`：消息记录页列场所会话、删她的上下文（施工 I-34 上），同样转给读的一头。
//! - `records.media`：消息记录页现取一条带的图片、文件（施工 C-1 中下补，`records_media`）。
//! - `records.venues`、`records.page`、`records.search`、`records.delete`：消息记录页读、搜、删 im 自己的聊天记录库（施工 C-1 中，`records_page`）。
//! - `models`：会话浮窗选模型（施工 I-7 中一，`venues::models`）。
//! - `reactions` → `{"reactions": [{id, name, glyph?}], "default"}`：能贴的表情（驱动给的，空的是这个平台不支持贴表情）和桥出厂
//!   贴的那一个（`bridge.json` 的 `reaction_emoji`，施工 I-7 中三下二）。
//! - `settings.get`、`settings.set`：设置页（施工 I-7 下一，`settings`）。
//! - `plugins.list`、`plugins.readme`、`plugins.enable`：插件页（施工 I-9 上，`plugins`）。
//! - `plugins.settings.get`、`plugins.settings.set`、`plugins.call`：插件自己的设置、插件面板（施工 I-9 中，`plugin_settings`）。
//! - `plugins.panel`：插件面板的地址（施工 I-9 下，`panels`）。
//! - `plugins.home`：首页上放不放这个插件的卡片（施工 I-8 下，`home_cards`）。
//! - `plugins.widget`：插件往页面加的内容（施工 I-8 下，`plugin_widgets`）。
//! - 别的：`unknown_method`。

use crate::chat::Id;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::logins::Login;
use super::{
    Person, Site, home_cards, logs, moderation, panels, plugin_settings, plugin_widgets, plugins,
    records_media, records_page, save, scope, settings, stats, store, venues,
};
use crate::core::methods::short_path;
use crate::running::Station;

/// 答方法 `method`（参数 `params`）：交回 `result`，不成的交回（原因、原话，原话可以空）。
pub(super) async fn answer(
    site: &Arc<Site>,
    login: &Login<Person>,
    (method, params): (Option<&str>, &Value),
) -> Result<Value, (String, String)> {
    match method {
        Some("connections") => Ok(json!({"connections": connections(site)})),
        Some("status") => {
            let (station, _) = site.pick(params)?;
            let mut status = site.running.status_on(station);
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
            let scope = scope::Scope::of(params)?;
            Ok(json!({"logs": logs::recent(&file, filter, limit, scope.as_ref())}))
        }
        Some("models") => venues::models(&login.person).await,
        Some("presets") => venues::presets(&login.person).await,
        Some("moderation.get") => moderation::get(site),
        Some("moderation.set") => moderation::set(site, params),
        Some("reactions") => reactions(site, params),
        Some("settings.get") => settings::get(&login.person).await,
        Some("settings.set") => settings::set(&login.person, params).await,
        Some("plugins.list") => plugins::list(site, &login.person).await,
        Some("plugins.home") => home_cards::set(site, &login.person, params).await,
        Some("plugins.widget") => plugin_widgets::widget(site, &login.person, params).await,
        Some("plugins.readme") => plugins::readme(site, &login.person, params).await,
        Some("plugins.enable") => plugins::enable(&login.person, params).await,
        Some("plugins.settings.get") => plugin_settings::get(&login.person, params).await,
        Some("plugins.settings.set") => plugin_settings::set(&login.person, params).await,
        Some("plugins.call") => plugin_settings::call(site, &login.person, params).await,
        Some("plugins.panel") => panels::open(site, login, params).await,
        Some("stats") => stats::stats(site, &login.person, params).await,
        Some("store.list") => store::list(&login.person, params).await,
        Some("store.install") => store::install(&login.person, params).await,
        Some("records.venues") => records_page::listed(site).await,
        Some("records.page") => records_page::page(site, params).await,
        Some("records.search") => records_page::search(site, params).await,
        Some("records.delete") => records_page::remove(site, params).await,
        Some("records.media") => records_media::media(site, params).await,
        Some(method) if venues::FORWARDED.contains(&method) => {
            venues::forward(&login.person, method, params).await
        }
        _ => Err(("unknown_method".to_string(), String::new())),
    }
}

/// 能贴的表情（模块的说明）：照 `platform` 那个驱动的（施工 I-85 上）。
fn reactions(site: &Arc<Site>, params: &Value) -> Result<Value, (String, String)> {
    let (_, driver) = site.pick(params)?;
    let reactions: Vec<Value> = driver
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
    Ok(json!({"reactions": reactions, "default": site.running.tuning.reaction_emoji}))
}

/// 机器人（模块的说明）：挂着的每个平台的，照驱动的先后（施工 I-85 上）。
fn connections(site: &Arc<Site>) -> Vec<Value> {
    let avatars = !site.running.tuning.avatar_user_url.is_empty();
    site.running
        .drivers
        .iter()
        .flat_map(|station| on_platform(site, station, avatars))
        .collect()
}

/// 驱动 `station` 连着的、断开了的机器人：每个带连接编号 `id`（`<平台>:<号>`）、平台做得到的几样 `caps`（施工 I-85 上）。
fn on_platform(site: &Arc<Site>, station: &Station, avatars: bool) -> Vec<Value> {
    let platform = station.platform.name();
    let caps: Vec<&str> = station.caps.iter().map(|cap| cap.name()).collect();
    let card = |bot: Id, connected: bool, since: SystemTime| {
        let mut one = Map::new();
        one.insert("id".into(), json!(format!("{platform}:{bot}")));
        one.insert("platform".into(), json!(platform));
        one.insert("bot".into(), json!(bot.to_string()));
        one.insert("connected".into(), json!(connected));
        one.insert("since".into(), json!(millis(since)));
        one.insert("caps".into(), json!(caps));
        if let Some(name) = site.faces.name(site, bot.clone()) {
            one.insert("name".into(), json!(name));
        }
        if avatars {
            one.insert("avatar".into(), json!(format!("/avatar/{bot}")));
        }
        one
    };
    let mut all: Vec<Value> = station
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
        station
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
