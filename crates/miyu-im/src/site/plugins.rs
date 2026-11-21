//! 插件页（施工 I-9 上，设计 01 第四节、第八节第 4 条）：
//!
//! - `plugins.list` → `{"plugins": [...]}`：im 认得的插件（会话浮窗的 `cards.get` 交的那一份，`crate::core::hooks`），再加关着的插件
//!   （施工 I-249，核心 UX-1：关着的包的内容核心不交，im 读不到它的 `plugin.toml`；照 `package.list` 里 `enabled` 是假、`provides` 的
//!   `content` 有 im 的列，只有包名、核心照连接的语言挑好的 `name` 和下面并上的几格），每个并上核心说的：
//!   `package.list` 的 `version`、`author`、`from`、`enabled`、`status`、`page`（有后台页的为真）和 `points`（导出的挂接点，照
//!   `POINTS` 的先后），`extension.status` 的 `state`、`reason`、`capabilities`（`[{id, name, summary}]`）、`unapproved`。核心那边没有
//!   这个包的只有 im 的几格。声明了首页卡片的另带 `on_home`（放不放，`home_cards`，施工 I-8 下）。
//! - `plugins.readme {package}` → `{"text"}`：插件声明的 README（`links.readme`，经桥自己的连接 `content.read`：只有 im 读得到插件带给
//!   它的目录）。没写的、读不成的、关着的 `not_found`，不是 UTF-8 的 `bad_readme`。
//! - `plugins.enable {package, on, approve?}` → 核心回的那一项：人的连接上 `package.enable`（`approve` 是要一并批准的能力的编号，
//!   照传；还有没批的核心拒 `needs_approval`）或 `package.disable`。不是 im 插件的（认得的、关着的都不是）、`approve` 不是字的列表
//!   的 `bad_params`。
//!
//! 驱动带的插件（施工 I-10 上，`builtin: true`）不是软件包：`plugins.list` 的 `enabled` 照它的总开关（配置项 `<settings>on`，没写的开），
//! `from` 是 `builtin`，版本、作者照 im 的包，挂在进站；`plugins.enable` 改那个配置项（写进系统那一层）。
//!
//! 读核心的都在这份登录的人的连接上（人的权限，设计第九节第 2 条）。

use serde_json::{Map, Value, json};

use super::{Person, Site, home_cards, venues};
use crate::core::Gone;
use crate::core::hooks::POINTS;
use crate::core::methods::CARDS_GET;
use crate::plugins::read_all;

/// 在 `person` 的连接上调核心的 `method`：拒了的交回（原因、原话）。
pub(super) async fn core(
    person: &Person,
    method: &str,
    params: Value,
) -> Result<Value, (String, String)> {
    person
        .lock()
        .await
        .call("plugins", method, params)
        .await
        .map_err(|why| (why.reason, why.message))
}

/// im 认得的插件（`cards.get` 的 `plugins`）。
pub(super) async fn known(person: &Person) -> Result<Vec<Value>, (String, String)> {
    let cards = venues::forward(person, CARDS_GET, &json!({})).await?;
    Ok(cards["plugins"].as_array().cloned().unwrap_or_default())
}

/// `package.list` 的回应 `packages` 里关着的 im 插件（施工 I-249）：`enabled` 是假、`provides` 的 `content` 有 im，不在 `known` 里的。
/// 每个只有 `package` 和核心挑好的 `name`。
pub(super) fn off(packages: &Value, known: &[Value]) -> Vec<Value> {
    packages["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| entry["enabled"] == false)
        .filter(|entry| {
            entry["provides"]["content"]
                .as_array()
                .is_some_and(|hosts| hosts.iter().any(|host| host == crate::PACKAGE))
        })
        .filter(|entry| {
            !known
                .iter()
                .any(|plugin| plugin["package"] == entry["package"])
        })
        .map(|entry| json!({"package": entry["package"], "name": entry["name"]}))
        .collect()
}

/// `package` 是不是关着的 im 插件（[`off`]）。
async fn is_off(person: &Person, package: &str, known: &[Value]) -> Result<bool, (String, String)> {
    let packages = core(person, "package.list", json!({})).await?;
    Ok(off(&packages, known)
        .iter()
        .any(|plugin| plugin["package"] == package))
}

/// `package` 是不是 im 认得的插件。
pub(super) async fn is_plugin(person: &Person, package: &str) -> Result<bool, (String, String)> {
    Ok(known(person)
        .await?
        .iter()
        .any(|plugin| plugin["package"] == package))
}

/// `plugins.list`（模块的说明）。
pub(super) async fn list(site: &Site, person: &Person) -> Result<Value, (String, String)> {
    let known = known(person).await?;
    let hidden = home_cards::hidden(&site.root);
    let packages = core(person, "package.list", json!({})).await?;
    let extensions = core(person, "extension.status", json!({})).await?;
    let switches: Vec<String> = known
        .iter()
        .filter(|plugin| plugin["builtin"] == true)
        .filter_map(|plugin| Some(format!("{}on", plugin["settings"].as_str()?)))
        .collect();
    let switched = match switches.is_empty() {
        true => json!({"items": {}}),
        false => core(person, "config.get", json!({"keys": switches})).await?,
    };
    let own = packages["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|entry| entry["package"] == crate::PACKAGE)
        .cloned()
        .unwrap_or(Value::Null);
    let closed = off(&packages, &known);
    let plugins: Vec<Value> = known
        .into_iter()
        .chain(closed)
        .map(|plugin| merged(plugin, &packages["packages"], &extensions["extensions"]))
        .map(|plugin| builtin(plugin, &switched, &own))
        .map(|mut plugin| {
            if plugin["ui"]["home"] == true {
                let package = plugin["package"].as_str().unwrap_or_default();
                plugin["on_home"] = json!(!hidden.contains(package));
            }
            plugin
        })
        .collect();
    Ok(json!({"plugins": plugins}))
}

/// 一个插件 `plugin` 并上 `package.list` 的 `packages`、`extension.status` 的 `extensions` 里它那一项（模块的说明）。
pub(super) fn merged(mut plugin: Value, packages: &Value, extensions: &Value) -> Value {
    let package = plugin["package"].as_str().unwrap_or_default().to_string();
    let find = |list: &Value| {
        list.as_array()
            .into_iter()
            .flatten()
            .find(|entry| entry["package"] == package.as_str())
            .cloned()
    };
    let Some(shown) = plugin.as_object_mut() else {
        return plugin;
    };
    if let Some(entry) = find(packages) {
        copy(
            shown,
            &entry,
            // 施工 I-272：上游（源码、主页的网址）照网页包的软件包详情写一行链接。
            &["version", "author", "from", "enabled", "status", "upstream"],
        );
        if entry
            .get("page")
            .is_some_and(|page| !page.is_null() && *page != false)
        {
            shown.insert("page".into(), json!(true));
        }
        let exported: Vec<&str> = entry["export"]["methods"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let points: Vec<&str> = POINTS
            .into_iter()
            .filter(|point| exported.contains(point))
            .collect();
        shown.insert("points".into(), json!(points));
    }
    if let Some(entry) = find(extensions) {
        copy(
            shown,
            &entry,
            &["state", "reason", "capabilities", "unapproved"],
        );
    }
    plugin
}

/// 驱动带的插件 `plugin`（不是的原样交回）：`enabled` 照 `config.get` 的回答 `switched` 里它的总开关（没有的开），`from` 是
/// `builtin`，版本、作者、上游（施工 I-272）照 im 的包 `own`（`package.list` 的那一项），挂在进站、在跑。
pub(super) fn builtin(mut plugin: Value, switched: &Value, own: &Value) -> Value {
    if plugin["builtin"] != true {
        return plugin;
    }
    let switch = format!("{}on", plugin["settings"].as_str().unwrap_or_default());
    let enabled = switched["items"][&switch]["value"]
        .as_bool()
        .unwrap_or(true);
    let Some(shown) = plugin.as_object_mut() else {
        return plugin;
    };
    copy(shown, own, &["version", "author", "upstream"]);
    shown.insert("enabled".into(), json!(enabled));
    shown.insert("from".into(), json!("builtin"));
    shown.insert("points".into(), json!(["chat.hook.inbound"]));
    shown.insert(
        "status".into(),
        json!(if enabled { "running" } else { "stopped" }),
    );
    plugin
}

/// 把 `from` 里写了的 `keys` 抄进 `into`。
fn copy(into: &mut Map<String, Value>, from: &Value, keys: &[&str]) {
    for key in keys {
        if let Some(value) = from.get(*key).filter(|value| !value.is_null()) {
            into.insert((*key).to_string(), value.clone());
        }
    }
}

/// `plugins.readme`（模块的说明）。
pub(super) async fn readme(
    site: &Site,
    person: &Person,
    params: &Value,
) -> Result<Value, (String, String)> {
    let package = params["package"].as_str().ok_or_else(bad)?;
    let known = known(person).await?;
    let Some(plugin) = known.iter().find(|plugin| plugin["package"] == package) else {
        // 关着的插件（施工 I-249）：im 读不到它带的目录。
        return Err(match is_off(person, package, &known).await? {
            true => ("not_found".to_string(), String::new()),
            false => bad(),
        });
    };
    let path = plugin["links"]["readme"]
        .as_str()
        .ok_or_else(|| ("not_found".to_string(), String::new()))?
        .to_string();
    let read = read_all(&site.caller, package, &path)
        .await
        .map_err(|Gone| ("program_not_running".to_string(), String::new()))?;
    let bytes = read.map_err(|why| {
        tracing::warn!(target: crate::TARGET, package, why, "plugin readme not read");
        ("not_found".to_string(), String::new())
    })?;
    let text = String::from_utf8(bytes).map_err(|_| ("bad_readme".to_string(), String::new()))?;
    Ok(json!({"text": text}))
}

/// `plugins.enable`（模块的说明）。
pub(super) async fn enable(person: &Person, params: &Value) -> Result<Value, (String, String)> {
    let package = params["package"].as_str().ok_or_else(bad)?;
    let on = params["on"].as_bool().ok_or_else(bad)?;
    let known = known(person).await?;
    let plugin = known.iter().find(|plugin| plugin["package"] == package);
    // 关着的插件（施工 I-249）照样开关得了，它不是驱动带的。
    if plugin.is_none() && !is_off(person, package, &known).await? {
        return Err(bad());
    }
    if let Some(plugin) = plugin.filter(|plugin| plugin["builtin"] == true) {
        let key = format!("{}on", plugin["settings"].as_str().ok_or_else(bad)?);
        let change = json!({"key": key, "value": on});
        core(
            person,
            "config.set",
            json!({"layer": "system", "changes": [change]}),
        )
        .await?;
        tracing::info!(target: crate::TARGET, package, on, "builtin plugin switched from the page");
        return Ok(json!({"package": package, "enabled": on}));
    }
    let answer = if on {
        let mut args = json!({"package": package});
        match &params["approve"] {
            Value::Null => {}
            Value::Array(ids) if ids.iter().all(Value::is_string) => {
                args["approve"] = json!(ids);
            }
            _ => return Err(bad()),
        }
        core(person, "package.enable", args).await?
    } else {
        core(person, "package.disable", json!({"package": package})).await?
    };
    tracing::info!(target: crate::TARGET, package, on, "plugin switched from the page");
    Ok(answer)
}

/// 参数不对。
pub(super) fn bad() -> (String, String) {
    ("bad_params".to_string(), String::new())
}

#[cfg(test)]
mod tests;
