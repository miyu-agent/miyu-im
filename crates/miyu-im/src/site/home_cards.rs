//! 首页上放不放插件的卡片（施工 I-8 下，设计 01 第四节第 5 条；2026-10-20 项目主人定：只在插件详情里开关「放在首页」，顺序照插件的
//! 编号，不能调）：声明了 `home` 的插件出厂放；关了的记在 `<数据根>/state/packages/miyu-im/home.json` 的 `hidden`（编号的列表），由页面服务
//! 读写。读不成的当都放、记一行运行日志；写先写临时文件再改名盖上。
//!
//! - `plugins.home {package, shown}` → `{"saved": true}`：不是 im 插件的、`shown` 不是布尔的 `bad_params`。

use std::collections::BTreeSet;
use std::path::PathBuf;

use miyu_client::places::DataRoot;
use serde_json::{Value, json};

use super::plugins::{bad, is_plugin};
use super::{Person, Site};
use crate::{PACKAGE, TARGET};

/// 文件名：在这个包放状态的目录里。
const FILE: &str = "home.json";

/// 数据根 `root` 里这份文件在哪。
fn path(root: &DataRoot) -> PathBuf {
    root.state().join("packages").join(PACKAGE).join(FILE)
}

/// 首页上不放的插件；没有这份文件、读不成的是空的。
pub(super) fn hidden(root: &DataRoot) -> BTreeSet<String> {
    let Ok(bytes) = std::fs::read(path(root)) else {
        return BTreeSet::new();
    };
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(read) => read["hidden"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|one| one.as_str().map(str::to_string))
            .collect(),
        Err(error) => {
            tracing::warn!(target: TARGET, error = %error, "home cards file unreadable");
            BTreeSet::new()
        }
    }
}

/// `plugins.home`（模块的说明）。
pub(super) async fn set(
    site: &Site,
    person: &Person,
    params: &Value,
) -> Result<Value, (String, String)> {
    let package = params["package"].as_str().ok_or_else(bad)?;
    let shown = params["shown"].as_bool().ok_or_else(bad)?;
    if !is_plugin(person, package).await? {
        return Err(bad());
    }
    let mut hidden = hidden(&site.root);
    if shown {
        hidden.remove(package);
    } else {
        hidden.insert(package.to_string());
    }
    let mut bytes = serde_json::to_vec_pretty(&json!({"hidden": hidden}))
        .map_err(|_| ("not_saved".to_string(), String::new()))?;
    bytes.push(b'\n');
    let target = path(&site.root);
    let written =
        tokio::task::spawn_blocking(move || miyu_client::places::write(&target, &bytes)).await;
    match written {
        Ok(Ok(_)) => {
            tracing::info!(target: TARGET, package, shown, "home card switched");
            Ok(json!({"saved": true}))
        }
        _ => Err(("not_saved".to_string(), String::new())),
    }
}
