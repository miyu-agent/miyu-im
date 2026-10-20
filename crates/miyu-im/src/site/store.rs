//! 商城（施工 I-11，设计 01 第一节第 6 条第三页；2026-10-20 项目主人定：只列 im 的插件、装好留在商城、非官方的只标标签）：经这份
//! 登录的人的连接在核心的软件源里搜、装。
//!
//! - `store.list {query?}` → `{"packages": [...], "stale"?}`：核心 `package.search` 里带 im 内容（`contents` 有 `im`）的那几项，照原样
//!   （名字、说明照连接的语言挑好，`official`、`installed`、`blocked`）。核心的错照原样交回（软件源连不上这类）。
//! - `store.install {package}` → `package.install {source}` 的回应：从软件源装（缺的依赖一起装）。装好以后照常在插件页开、批能力。

use serde_json::{Value, json};

use super::Person;
use super::plugins::{bad, core};

/// `store.list`（模块的说明）。
pub(super) async fn list(person: &Person, params: &Value) -> Result<Value, (String, String)> {
    let mut args = json!({});
    if let Some(query) = params["query"]
        .as_str()
        .map(str::trim)
        .filter(|query| !query.is_empty())
    {
        args["query"] = json!(query);
    }
    let found = core(person, "package.search", args).await?;
    let mut answer = json!({"packages": plugins(&found["packages"])});
    if found.get("stale").is_some_and(|stale| !stale.is_null()) {
        answer["stale"] = found["stale"].clone();
    }
    Ok(answer)
}

/// 搜到的 `packages` 里 im 的插件（纯逻辑）：`contents` 有 `im` 的。
pub(super) fn plugins(packages: &Value) -> Vec<Value> {
    packages
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| {
            item["contents"]
                .as_array()
                .is_some_and(|contents| contents.iter().any(|kind| kind == "im"))
        })
        .cloned()
        .collect()
}

/// 一个包的编号（纯逻辑，照核心 `packages.md` 的写法：小写字母开头，小写字母、数字、`-`、`_`，最多 64 个）。核心的
/// `package.install` 也收路径、地址，商城只装软件源里的，别的拒。
pub(super) fn named(package: &str) -> bool {
    package.len() <= 64
        && package
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_lowercase())
        && package
            .chars()
            .all(|one| one.is_ascii_lowercase() || one.is_ascii_digit() || one == '-' || one == '_')
}

/// `store.install`（模块的说明）。
pub(super) async fn install(person: &Person, params: &Value) -> Result<Value, (String, String)> {
    let package = params["package"]
        .as_str()
        .filter(|package| named(package))
        .ok_or_else(bad)?;
    let installed = core(person, "package.install", json!({"source": package})).await?;
    tracing::info!(target: crate::TARGET, package, "plugin installed from the store");
    Ok(installed)
}

#[cfg(test)]
mod tests;
