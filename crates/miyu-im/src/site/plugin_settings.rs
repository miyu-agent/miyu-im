//! 插件自己的设置、插件面板调它的程序（施工 I-9 中，设计 01 第四节第 5 条、第九节第 5 条；照核心 `package-pages.md`「框和网页之间怎么说」
//! 的 `settings.get`、`settings.set`、`call`，同 WebUI 的 `backstage/frame.js`）。插件详情的「设置」、沙箱框里的插件面板都用它们。
//!
//! - `plugins.settings.get {package}` → `{"items": [...], "values": {键: 最终值}}`：`config.schema` 里 `package` 是它的项（照清单的先后），
//!   `config.get` 的最终值，没写的照默认；密钥照 `config.get` 的写法（是引用，不是值）。
//! - `plugins.settings.set {package, changes}` → `config.set` 的回应：每一项的键都要是 `<编号>.` 开头，有一项不是的整个不办（`forbidden`）；
//!   密钥类型给的是一段字的，先 `secret.set` 存成一个新名字的密钥（照键起名、带上时刻，最多 64 个字，不覆盖原来的），再写引用。照
//!   `config.set` 写进系统那一层。
//! - `plugins.call {package, method, params?}` → `package.call` 的回应；驱动带的插件在进程里直接问（施工 Q-6，`site/builtins.rs`）。
//!
//! 都在这份登录的人的连接上（人的权限）；`package` 由页面填（框里的页面写不了），不是 im 插件的不收（`bad_params`）。驱动带的插件
//! （施工 I-10 上）的设置是 im 的配置项、键以它的 `settings` 开头（藏起来的，这里照常列，总开关 `<开头>on` 不列）。

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::plugins::{bad, core, known};
use super::{Person, Site};

/// `plugins.settings.get`（模块的说明）。
pub(super) async fn get(person: &Person, params: &Value) -> Result<Value, (String, String)> {
    let scope = plugin(person, params).await?;
    let items = items(person, &scope).await?;
    let keys: Vec<&Value> = items.iter().map(|item| &item["key"]).collect();
    let got = match keys.is_empty() {
        true => json!({"items": {}}),
        false => core(person, "config.get", json!({"keys": keys})).await?,
    };
    Ok(json!({"items": items, "values": values(&items, &got)}))
}

/// `config.schema` 的 `items` 和 `config.get` 的回答 `got` 拼出每一项的最终值：没写的照默认，都没有的是 `null`。
pub(super) fn values(items: &[Value], got: &Value) -> Value {
    let values: serde_json::Map<String, Value> = items
        .iter()
        .filter_map(|item| {
            let key = item["key"].as_str()?;
            let value = got["items"][key]
                .get("value")
                .or_else(|| item.get("default"))
                .cloned()
                .unwrap_or(Value::Null);
            Some((key.to_string(), value))
        })
        .collect();
    Value::Object(values)
}

/// `plugins.settings.set`（模块的说明）。
pub(super) async fn set(person: &Person, params: &Value) -> Result<Value, (String, String)> {
    let scope = plugin(person, params).await?;
    let package = scope.package.clone();
    let changes = params["changes"]
        .as_array()
        .filter(|changes| !changes.is_empty())
        .ok_or_else(bad)?;
    if changes.iter().any(|change| !change["key"].is_string()) {
        return Err(bad());
    }
    if !all_own(&scope.prefix, changes) {
        return Err(("forbidden".to_string(), String::new()));
    }
    let items = items(person, &scope).await?;
    let mut written = Vec::new();
    for change in changes {
        let key = change["key"].as_str().unwrap_or_default();
        let secret = items
            .iter()
            .any(|item| item["key"] == key && item["type"] == "secret");
        match (&change["value"], secret) {
            (Value::String(text), true) => {
                let name = secret_name(key, now_millis());
                core(person, "secret.set", json!({"name": name, "value": text})).await?;
                written.push(json!({"key": key, "value": {"secret": name}}));
            }
            _ => written.push(change.clone()),
        }
    }
    let answer = core(
        person,
        "config.set",
        json!({"layer": "system", "changes": written}),
    )
    .await?;
    tracing::info!(target: crate::TARGET, package, count = written.len(), "plugin settings saved from the page");
    Ok(answer)
}

/// `plugins.call`（模块的说明）。
pub(super) async fn call(
    site: &Site,
    person: &Person,
    params: &Value,
) -> Result<Value, (String, String)> {
    let scope = plugin(person, params).await?;
    let method = params["method"].as_str().ok_or_else(bad)?;
    if scope.builtin {
        // 驱动带的插件在进程里直接问（施工 Q-6）。
        let given = params.get("params").cloned().unwrap_or(Value::Null);
        return site.call_builtin(&scope.package, method, &given).await;
    }
    let mut args = json!({"package": scope.package, "method": method});
    if let Some(given) = params.get("params").filter(|given| !given.is_null()) {
        args["params"] = given.clone();
    }
    core(person, "package.call", args).await
}

/// 一个插件的设置在哪：插件的编号、它的配置项的键的开头（软件包是 `<编号>.`，驱动带的是它的 `settings`）、是不是驱动带的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Scope {
    pub(super) package: String,
    pub(super) prefix: String,
    pub(super) builtin: bool,
}

/// 参数里的 `package`：要是 im 认得的插件。
async fn plugin(person: &Person, params: &Value) -> Result<Scope, (String, String)> {
    let package = params["package"].as_str().ok_or_else(bad)?;
    let known = known(person).await?;
    let plugin = known
        .iter()
        .find(|plugin| plugin["package"] == package)
        .ok_or_else(bad)?;
    Ok(
        match (plugin["builtin"] == true, plugin["settings"].as_str()) {
            (true, Some(prefix)) => Scope {
                package: package.to_string(),
                prefix: prefix.to_string(),
                builtin: true,
            },
            _ => Scope {
                package: package.to_string(),
                prefix: format!("{package}."),
                builtin: false,
            },
        },
    )
}

/// `config.schema` 里这个插件的项。
async fn items(person: &Person, scope: &Scope) -> Result<Vec<Value>, (String, String)> {
    let schema = core(person, "config.schema", json!({})).await?;
    Ok(own_items(&schema["items"], scope))
}

/// `config.schema` 的 `items` 里 `scope` 的项：软件包的照 `package`，键还要是 `<编号>.` 开头（核心给每个包加的「只对你自己关掉」
/// `packages.<编号>.off` 也标着这个包，它不是插件自己的设置，这一页也存不了它，施工 I-38 换钉时撞上的）；驱动带的照键的开头，总开关
/// 不列，`hidden` 拿掉（只在插件页改）。
pub(super) fn own_items(items: &Value, scope: &Scope) -> Vec<Value> {
    let switch = format!("{}on", scope.prefix);
    items
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| match scope.builtin {
            true => item["key"]
                .as_str()
                .is_some_and(|key| key.starts_with(&scope.prefix) && key != switch),
            false => {
                item["package"] == scope.package.as_str()
                    && item["key"]
                        .as_str()
                        .is_some_and(|key| key.starts_with(&scope.prefix))
            }
        })
        .map(|item| {
            let mut item = item.clone();
            if scope.builtin
                && let Some(fields) = item.as_object_mut()
            {
                fields.remove("hidden");
            }
            item
        })
        .collect()
}

/// `changes` 里每一项的键都是 `prefix` 开头（软件包的是 `<编号>.`）。
pub(super) fn all_own(prefix: &str, changes: &[Value]) -> bool {
    changes.iter().all(|change| {
        change["key"]
            .as_str()
            .is_some_and(|key| key.starts_with(prefix))
    })
}

/// 新密钥的名字：照键起（小写，别的字都换成 `-`），带上时刻 `millis`（36 进制），最多 64 个字（同 WebUI 的 `secretName`）。
pub(super) fn secret_name(key: &str, millis: u128) -> String {
    let mut name = String::new();
    for c in key.chars().flat_map(char::to_lowercase) {
        match c.is_ascii_lowercase() || c.is_ascii_digit() {
            true => name.push(c),
            false if !name.ends_with('-') => name.push('-'),
            false => {}
        }
    }
    name.push('-');
    name.push_str(&base36(millis));
    name.chars().take(64).collect()
}

/// `n` 写成 36 进制。
fn base36(mut n: u128) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    loop {
        out.push(DIGITS[usize::try_from(n % 36).unwrap_or(0)]);
        n /= 36;
        if n == 0 {
            break;
        }
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// 这一刻，毫秒的 Unix 时间。
pub(super) fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis())
}

#[cfg(test)]
mod tests;
