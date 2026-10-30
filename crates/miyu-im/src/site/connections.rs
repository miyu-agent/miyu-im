//! 连接表（施工 I-81 下中，设计 07 第 7.7 节；项目主人 10-30 定：连接只在 im 后台页改，Token 是 NapCat 生成的、每个连接一个）：
//! 后台页新建连接、连接卡齿轮里改、删一个连接。连接表是驱动的三个藏起来的配置项（`hosts`、`ports`、`tokens`，第几项是第几个连接），
//! 这里在这份登录的人的连接上替人读、写（人的权限）。
//!
//! - `connection.add {platform?, port, token, host?}` → `{"saved": true}`：加一个连接。端口 1024 到 65535、不能和表里的重；Token
//!   必写（NapCat 那边生成的）；地址不写的照驱动的默认。
//! - `connection.edit {platform?, port, values: {host?, listen?, token?}}` → `{"saved": true}`：改端口是 `port` 的那一个。`listen` 是新
//!   端口（不能和别的重）；`token` 空的不改。
//! - `connection.remove {platform?, port}` → `{"saved": true}`：删端口是 `port` 的那一个；表里只剩它的不删（`last_connection`）。
//!
//! 读：`config.get` 三个列表（密钥是引用，不是值）。表还没写过的（原来只有一个连接）：原来的 `host`、`listen`、`token` 设了 Token 的
//! 当头一个连接，没设 Token 的不算（新装还没配过，新建的就是头一个）。写：Token 先 `secret.set` 存成新名字的密钥（照键起名、带上
//! 时刻），三个列表一起写进系统配置（一次 `config.set`），原来的三项一起去掉。核心收下推 `extension.config`，驱动当场换。
//! 写错的（端口重了、找不到、Token 没写）：`bad_params`；核心拒了的照它的原因和原话交回。

use serde_json::{Value, json};

use super::plugin_settings::{now_millis, secret_name};
use super::{Person, Site};
use crate::settings::key;

/// 连接表的三列、原来的三项：清单里的名字。
const HOSTS: &str = "hosts";
const PORTS: &str = "ports";
const TOKENS: &str = "tokens";
const HOST: &str = "host";
const LISTEN: &str = "listen";
const TOKEN: &str = "token";

/// 表里的一个连接：地址、端口、Token 的引用（`{"secret": …}` 这类，原样写回）。
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Entry {
    pub(super) host: String,
    pub(super) port: u64,
    pub(super) token: Value,
}

/// 写错了。
fn bad() -> (String, String) {
    ("bad_params".to_string(), String::new())
}

/// 合不合端口的写法：1024 到 65535。
fn port_ok(port: u64) -> bool {
    (1024..=65535).contains(&port)
}

/// `config.get` 的回答 `got` 里平台 `platform` 的连接表（模块的说明「读」）。`fallback` 是驱动默认的地址、端口。
pub(super) fn table(got: &Value, platform: &str, fallback: (&str, u64)) -> Vec<Entry> {
    let value = |name: &str| got["items"][key(platform, name)]["value"].clone();
    let text = |value: &Value| value.as_str().map(str::to_string);
    if let Value::Array(ports) = value(PORTS) {
        let hosts = value(HOSTS);
        let tokens = value(TOKENS);
        return ports
            .iter()
            .enumerate()
            .filter_map(|(at, port)| {
                Some(Entry {
                    host: text(&hosts[at]).unwrap_or_else(|| fallback.0.to_string()),
                    port: port.as_u64()?,
                    token: tokens.get(at).cloned().filter(|token| !token.is_null())?,
                })
            })
            .collect();
    }
    let token = value(TOKEN);
    if token.is_null() {
        return Vec::new();
    }
    vec![Entry {
        host: text(&value(HOST)).unwrap_or_else(|| fallback.0.to_string()),
        port: value(LISTEN).as_u64().unwrap_or(fallback.1),
        token,
    }]
}

/// 照 `entries` 写的那几项：三个列表，加上去掉原来的三项。
pub(super) fn changes(platform: &str, entries: &[Entry]) -> Vec<Value> {
    let column = |pick: fn(&Entry) -> Value| entries.iter().map(pick).collect::<Vec<_>>();
    vec![
        json!({"key": key(platform, HOSTS), "value": column(|entry| json!(entry.host))}),
        json!({"key": key(platform, PORTS), "value": column(|entry| json!(entry.port))}),
        json!({"key": key(platform, TOKENS), "value": column(|entry| entry.token.clone())}),
        json!({"key": key(platform, HOST), "unset": true}),
        json!({"key": key(platform, LISTEN), "unset": true}),
        json!({"key": key(platform, TOKEN), "unset": true}),
    ]
}

/// 在这份登录的人的连接上调核心的 `method`。
async fn call(person: &Person, method: &str, params: Value) -> Result<Value, (String, String)> {
    person
        .lock()
        .await
        .call("config", method, params)
        .await
        .map_err(|why| (why.reason, why.message))
}

/// 读这一刻的连接表：平台名、表。
async fn read(
    site: &Site,
    person: &Person,
    params: &Value,
) -> Result<(String, Vec<Entry>), (String, String)> {
    let (station, _) = site.pick(params)?;
    let platform = station.platform.name().to_string();
    let keys: Vec<String> = [HOSTS, PORTS, TOKENS, HOST, LISTEN, TOKEN]
        .iter()
        .map(|name| key(&platform, name))
        .collect();
    let got = call(person, "config.get", json!({"keys": keys})).await?;
    let current = &site.running.current;
    let host = current
        .driver_default(station.platform, HOST)
        .and_then(|host| host.as_str().map(str::to_string))
        .unwrap_or_else(|| "0.0.0.0".to_string());
    let port = current
        .driver_default(station.platform, LISTEN)
        .and_then(|port| port.as_u64())
        .unwrap_or(8301);
    let entries = table(&got, &platform, (&host, port));
    Ok((platform, entries))
}

/// 一段 Token 先存成密钥，交回引用。
async fn stored(
    person: &Person,
    platform: &str,
    port: u64,
    text: &str,
) -> Result<Value, (String, String)> {
    let name = secret_name(
        &format!("{}-{platform}-token-{port}", crate::PREFIX),
        now_millis(),
    );
    call(
        person,
        "secret.set",
        json!({"name": name, "value": text.trim()}),
    )
    .await?;
    Ok(json!({"secret": name}))
}

/// 写回连接表。
async fn write(
    person: &Person,
    platform: &str,
    entries: &[Entry],
) -> Result<Value, (String, String)> {
    let changes = changes(platform, entries);
    call(
        person,
        "config.set",
        json!({"layer": "system", "changes": changes}),
    )
    .await?;
    tracing::info!(target: crate::TARGET, connections = entries.len(), "connection table saved");
    Ok(json!({"saved": true}))
}

/// `connection.add`（模块的说明）。
pub(super) async fn add(
    site: &Site,
    person: &Person,
    params: &Value,
) -> Result<Value, (String, String)> {
    let port = params["port"]
        .as_u64()
        .filter(|port| port_ok(*port))
        .ok_or_else(bad)?;
    let token = params["token"]
        .as_str()
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(bad)?;
    let (platform, mut entries) = read(site, person, params).await?;
    if entries.iter().any(|entry| entry.port == port) {
        return Err(bad());
    }
    let host = match &params["host"] {
        Value::Null => None,
        Value::String(host) if !host.trim().is_empty() => Some(host.trim().to_string()),
        _ => return Err(bad()),
    };
    let default_host = entries
        .first()
        .map_or_else(|| "0.0.0.0".to_string(), |entry| entry.host.clone());
    let token = stored(person, &platform, port, token).await?;
    entries.push(Entry {
        host: host.unwrap_or(default_host),
        port,
        token,
    });
    write(person, &platform, &entries).await
}

/// `connection.edit`（模块的说明）。
pub(super) async fn edit(
    site: &Site,
    person: &Person,
    params: &Value,
) -> Result<Value, (String, String)> {
    let port = params["port"].as_u64().ok_or_else(bad)?;
    let values = params["values"].as_object().ok_or_else(bad)?;
    let (platform, mut entries) = read(site, person, params).await?;
    let at = entries
        .iter()
        .position(|entry| entry.port == port)
        .ok_or_else(bad)?;
    if let Some(host) = values.get(HOST) {
        let host = host
            .as_str()
            .map(str::trim)
            .filter(|host| !host.is_empty())
            .ok_or_else(bad)?;
        entries[at].host = host.to_string();
    }
    if let Some(listen) = values.get(LISTEN) {
        let listen = listen
            .as_u64()
            .or_else(|| listen.as_str().and_then(|text| text.trim().parse().ok()))
            .filter(|listen| port_ok(*listen))
            .ok_or_else(bad)?;
        if entries
            .iter()
            .enumerate()
            .any(|(other, entry)| other != at && entry.port == listen)
        {
            return Err(bad());
        }
        entries[at].port = listen;
    }
    if let Some(token) = values.get(TOKEN) {
        let token = token.as_str().ok_or_else(bad)?;
        if !token.trim().is_empty() {
            entries[at].token = stored(person, &platform, entries[at].port, token).await?;
        }
    }
    write(person, &platform, &entries).await
}

/// `connection.remove`（模块的说明）。
pub(super) async fn remove(
    site: &Site,
    person: &Person,
    params: &Value,
) -> Result<Value, (String, String)> {
    let port = params["port"].as_u64().ok_or_else(bad)?;
    let (platform, mut entries) = read(site, person, params).await?;
    let at = entries
        .iter()
        .position(|entry| entry.port == port)
        .ok_or_else(bad)?;
    if entries.len() == 1 {
        return Err(("last_connection".to_string(), String::new()));
    }
    entries.remove(at);
    write(person, &platform, &entries).await
}

#[cfg(test)]
mod tests;
