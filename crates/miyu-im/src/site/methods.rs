//! 页面经通道调的方法（施工 I-6 上）：照这一刻桥手里的样子答，不读盘。
//!
//! - `whoami` → `{"account"}`：这份登录的账号。
//! - `connections` → `{"connections": [...]}`：连着的每一个机器人，照号排：`platform`（驱动给的平台名）、`bot`（号，写成字）、
//!   `since`（连上的时刻，毫秒的 Unix 时间）、问到了是哪个实现的再带 `implementation`、`version`。
//! - 别的：`unknown_method`。

use std::time::UNIX_EPOCH;

use serde_json::{Map, Value, json};

use super::Site;

/// 答方法 `method`：交回 `result`，不认识的交回原因。
pub(super) fn answer(
    site: &Site,
    account: &str,
    method: Option<&str>,
) -> Result<Value, &'static str> {
    match method {
        Some("whoami") => Ok(json!({"account": account})),
        Some("connections") => Ok(json!({"connections": connections(site)})),
        _ => Err("unknown_method"),
    }
}

/// 连着的每一个机器人（模块的说明）。
fn connections(site: &Site) -> Vec<Value> {
    site.bots
        .held()
        .into_iter()
        .map(|(bot, link, since)| {
            let mut one = Map::new();
            one.insert("platform".into(), json!(site.platform.name()));
            one.insert("bot".into(), json!(bot.to_string()));
            let since = since.duration_since(UNIX_EPOCH).map_or(0, |since| {
                u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
            });
            one.insert("since".into(), json!(since));
            if let Some(implementation) = link.implementation() {
                one.insert("implementation".into(), json!(implementation.name));
                one.insert("version".into(), json!(implementation.version));
            }
            Value::Object(one)
        })
        .collect()
}
