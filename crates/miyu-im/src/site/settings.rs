//! 设置页（施工 I-7 下一，设计 01 第八节第 6 条）：通用（回执自动撤回的命令；去掉末尾句号、模拟真人一条分多条发送施工 I-49 起挪到会话
//! 卡片的「发送消息处理」，每张卡片单独设）、网页服务的端口、本人的平台 ID（核心的 `external.bindings`：键是平台身份，值不空就算
//! 主人本人，核心 SU-3 下起，施工 I-68）。都是核心的配置，在这份登录的人的连接上读、改，页面服务只让改下面这几项。
//!
//! - `settings.get` → `{"recall_receipts", "port", "listen", "lan", "lans", "macos", "bindings": [身份]}`：照核心的 `config.get`，没写的照清单的默认；对应表只交值不空的
//!   身份（值是什么不交：只有一个主人，写 `owner` 还是老的 `admin` 都一样）。`listen` 是 `im.web.listen`（施工 I-167，
//!   [`super::listen`]），`lan` 是现在听着的局域网网址的地址和端口（`192.168.1.2:8302`），只听本机、问不到的是空的；
//!   `lans` 是每块网卡的（`[{"address", "card"}]`，施工 I-193），`lan` 是其中第一个；`macos` 是 im 跑在 macOS 上（设置页写防火墙怎么放行）。
//! - `settings.set {key, value}` → `{"saved": true}`：`key` 是 `recall_receipts`、`port`、`listen`（只收 `127.0.0.1`、`0.0.0.0`），或者 `binding:<身份>`（`value` 是 `true` 的
//!   记成主人，写 `owner`；`null` 的去掉那一条；别的 `bad_params`）。身份得是 `<平台>:<号>`（施工 I-85 补：页面能选别的平台了），号照
//!   宿主的规矩（施工 I-79），不是的 `bad_params`。写进系统那一层（`config.set`），核心拒了的照它的原因和原话交回；
//!   别的键 `bad_params`。

use serde_json::{Value, json};

use std::net::{IpAddr, SocketAddr};

use super::Person;
use super::listen::{LISTEN, lan_addresses, valid};
use crate::chat::Id;
use crate::rules::platform_ok;
use crate::settings::{SITE_GROUP, key};

/// 对应表的键的开头。
const BINDINGS: &str = "external.bindings.";

/// 页面加的身份在对应表里写的值（核心 SU-3 下：值不空就算主人，搬家脚本也写它）。
const OWNER: &str = "owner";

/// 页面上的名字到配置的键：回执自动撤回的命令、网页服务的端口。
fn config_key(name: &str) -> Option<String> {
    match name {
        "recall_receipts" => Some(format!("{}.{name}", crate::PREFIX)),
        "port" => Some(key(SITE_GROUP, "port")),
        LISTEN => Some(key(SITE_GROUP, LISTEN)),
        _ => None,
    }
}

/// `settings.get`（模块的说明）。
pub(super) async fn get(
    person: &Person,
    listening: Option<(u16, IpAddr)>,
) -> Result<Value, (String, String)> {
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
    let bindings: Vec<&str> = items
        .iter()
        .filter(|(_, item)| {
            item["value"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        })
        .filter_map(|(key, _)| {
            let identity = key.strip_prefix(BINDINGS)?;
            // 核心照 TOML 的写法给键加了引号（`"qq:10001"`）：页面上写不带引号的。
            Some(
                identity
                    .strip_prefix('"')
                    .and_then(|rest| rest.strip_suffix('"'))
                    .unwrap_or(identity),
            )
        })
        .collect();
    Ok(json!({
        "recall_receipts": value("recall_receipts"),
        "port": value("port"),
        "listen": value(LISTEN),
        "lan": lan(listening),
        "lans": lans(listening),
        "macos": cfg!(target_os = "macos"),
        "bindings": bindings,
    }))
}

/// `settings.set`（模块的说明）。
pub(super) async fn set(person: &Person, params: &Value) -> Result<Value, (String, String)> {
    let bad = || ("bad_params".to_string(), String::new());
    let name = params["key"].as_str().ok_or_else(bad)?;
    let change = match name.strip_prefix("binding:") {
        // 身份里有 `:`，照 TOML 的写法加引号（同核心交出来的键）。
        Some(identity) if identity_ok(identity) => {
            let key = format!("{BINDINGS}\"{identity}\"");
            match &params["value"] {
                Value::Null => json!({"key": key, "unset": true}),
                Value::Bool(true) => json!({"key": key, "value": OWNER}),
                _ => return Err(bad()),
            }
        }
        Some(_) => return Err(bad()),
        None => {
            let key = config_key(name).ok_or_else(bad)?;
            if name == LISTEN && !params["value"].as_str().is_some_and(valid) {
                return Err(bad());
            }
            match &params["value"] {
                Value::Null => json!({"key": key, "unset": true}),
                value => json!({"key": key, "value": value}),
            }
        }
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

/// 现在听着的局域网网址的地址和端口（模块的说明）：[`lans`] 的第一个；只听本机、问不到的是空的。
fn lan(listening: Option<(u16, IpAddr)>) -> Value {
    lans(listening)
        .as_array()
        .and_then(|all| all.first())
        .map_or(Value::Null, |first| first["address"].clone())
}

/// 局域网里打得开的全部地址（施工 I-193）：`[{"address": "<地址>:<端口>", "card": <网卡名> | null}]`，照网卡的先后。
fn lans(listening: Option<(u16, IpAddr)>) -> Value {
    let Some((port, ip)) = listening else {
        return json!([]);
    };
    lan_addresses(ip)
        .into_iter()
        .map(|(card, ip)| json!({"address": SocketAddr::new(ip, port).to_string(), "card": card}))
        .collect()
}

/// 身份 `identity` 写得对（施工 I-85 补）：`<平台>:<号>`，平台照卡片的规矩（拼得成场所编号），号照 [`Id`] 的规矩。
fn identity_ok(identity: &str) -> bool {
    identity
        .split_once(':')
        .is_some_and(|(platform, number)| platform_ok(platform) && Id::new(number).is_some())
}
