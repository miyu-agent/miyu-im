//! 插件页（施工 I-9 上，设计 01 第四节、第八节第 4 条）：
//!
//! - `plugins.list` → `{"plugins": [...]}`：im 认得的插件（会话浮窗的 `cards.get` 交的那一份，`crate::core::hooks`），每个并上核心说的：
//!   `package.list` 的 `version`、`author`、`from`、`enabled`、`status`、`page`（有后台页的为真）和 `points`（导出的挂接点，照
//!   `POINTS` 的先后），`extension.status` 的 `state`、`reason`、`capabilities`（`[{id, name, summary}]`）、`unapproved`。核心那边没有
//!   这个包的只有 im 的几格。
//! - `plugins.readme {package}` → `{"text"}`：插件声明的 README（`links.readme`，经桥自己的连接 `content.read`：只有 im 读得到插件带给
//!   它的目录）。没写的、读不成的 `not_found`，不是 UTF-8 的 `bad_readme`。
//! - `plugins.enable {package, on, approve?}` → 核心回的那一项：人的连接上 `package.enable`（`approve` 是要一并批准的能力的编号，
//!   照传；还有没批的核心拒 `needs_approval`）或 `package.disable`。不是 im 插件的、`approve` 不是字的列表的 `bad_params`。
//!
//! 读核心的都在这份登录的人的连接上（人的权限，设计第九节第 2 条）。

use serde_json::{Map, Value, json};

use super::{Person, Site, venues};
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
async fn known(person: &Person) -> Result<Vec<Value>, (String, String)> {
    let cards = venues::forward(person, CARDS_GET, &json!({})).await?;
    Ok(cards["plugins"].as_array().cloned().unwrap_or_default())
}

/// `package` 是不是 im 认得的插件。
pub(super) async fn is_plugin(person: &Person, package: &str) -> Result<bool, (String, String)> {
    Ok(known(person)
        .await?
        .iter()
        .any(|plugin| plugin["package"] == package))
}

/// `plugins.list`（模块的说明）。
pub(super) async fn list(person: &Person) -> Result<Value, (String, String)> {
    let known = known(person).await?;
    let packages = core(person, "package.list", json!({})).await?;
    let extensions = core(person, "extension.status", json!({})).await?;
    let plugins: Vec<Value> = known
        .into_iter()
        .map(|plugin| merged(plugin, &packages["packages"], &extensions["extensions"]))
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
            &["version", "author", "from", "enabled", "status"],
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
    let path = known
        .iter()
        .find(|plugin| plugin["package"] == package)
        .ok_or_else(bad)?["links"]["readme"]
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
    if !is_plugin(person, package).await? {
        return Err(bad());
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
