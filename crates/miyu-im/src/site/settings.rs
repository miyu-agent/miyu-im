//! 设置页（施工 I-7 下一，设计 01 第八节第 6 条）：通用（去掉末尾句号、模拟真人一条分多条发送、回执自动撤回的命令）、网页服务的端口、账号绑定（核心的
//! `external.bindings`：平台身份 → 本机账号）。都是核心的配置，在这份登录的人的连接上读、改（人的权限，设计第九节第 2 条），页面服务
//! 只让改下面这几项。
//!
//! - `settings.get` → `{"strip_period", "humanlike", "recall_receipts", "port", "bindings": {身份: 账号}, "account"}`：照核心的 `config.get`，没写的照清单
//!   的默认；`account` 是这份登录的账号（页面拿它当绑定的默认）。
//! - `settings.set {key, value}` → `{"saved": true}`：`key` 是 `strip_period`、`humanlike`、`recall_receipts`、`port`，或者 `binding:<身份>`（`value` 是
//!   `null` 的去掉那一条）。写进系统那一层（`config.set`），核心拒了的照它的原因和原话交回；别的键 `bad_params`。

use serde_json::{Map, Value, json};

use super::Person;
use crate::settings::{SITE_GROUP, key};

/// 绑定那张表的键的开头。
const BINDINGS: &str = "external.bindings.";

/// 页面上的名字到配置的键：去掉末尾句号、模拟真人一条分多条发送、回执自动撤回的命令、网页服务的端口。
fn config_key(name: &str) -> Option<String> {
    match name {
        "strip_period" | "humanlike" | "recall_receipts" => {
            Some(format!("{}.{name}", crate::PACKAGE))
        }
        "port" => Some(key(SITE_GROUP, "port")),
        _ => None,
    }
}

/// `settings.get`（模块的说明）。
pub(super) async fn get(person: &Person, account: &str) -> Result<Value, (String, String)> {
    let got = person
        .lock()
        .await
        .call("settings", "config.get", json!({}))
        .await
        .map_err(|why| (why.reason, why.message))?;
    let items = got["items"].as_object().cloned().unwrap_or_default();
    let value = |name: &str| {
        config_key(name)
            .and_then(|key| items.get(&key))
            .map_or(Value::Null, |item| item["value"].clone())
    };
    let bindings: Map<String, Value> = items
        .iter()
        .filter_map(|(key, item)| {
            let identity = key.strip_prefix(BINDINGS)?;
            // 核心照 TOML 的写法给键加了引号（`"qq:10001"`）：页面上写不带引号的。
            let identity = identity
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
                .unwrap_or(identity);
            Some((identity.to_string(), item["value"].clone()))
        })
        .collect();
    Ok(json!({
        "strip_period": value("strip_period"),
        "humanlike": value("humanlike"),
        "recall_receipts": value("recall_receipts"),
        "port": value("port"),
        "bindings": bindings,
        "account": account,
    }))
}

/// `settings.set`（模块的说明）。
pub(super) async fn set(person: &Person, params: &Value) -> Result<Value, (String, String)> {
    let bad = || ("bad_params".to_string(), String::new());
    let name = params["key"].as_str().ok_or_else(bad)?;
    let key = match name.strip_prefix("binding:") {
        // 身份里有 `:`，照 TOML 的写法加引号（同核心交出来的键）。
        Some(identity) if !identity.is_empty() && !identity.contains('"') => {
            format!("{BINDINGS}\"{identity}\"")
        }
        Some(_) => return Err(bad()),
        None => config_key(name).ok_or_else(bad)?,
    };
    let change = match &params["value"] {
        Value::Null => json!({"key": key, "unset": true}),
        value => json!({"key": key, "value": value}),
    };
    person
        .lock()
        .await
        .call(
            "settings",
            "config.set",
            json!({"layer": "system", "changes": [change]}),
        )
        .await
        .map_err(|why| (why.reason, why.message))?;
    tracing::info!(target: crate::TARGET, key = name, "setting saved from the page");
    Ok(json!({"saved": true}))
}
