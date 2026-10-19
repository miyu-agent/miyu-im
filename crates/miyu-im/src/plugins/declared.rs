//! 插件的声明 `plugin.toml`（施工 I-8 上，设计 01 第四节第 2 条）：插件带给 im 的那个目录的根上。只有一张 `[plugin]`：
//!
//! - `name`：显示名，照语言写（`zh`、`en`、`ja`），至少一种；`summary`：一句说明，同样照语言写。
//! - `icon`、`homepage`、`repository`、`issues`、`readme`：图标、主页、仓库、问题反馈、README 的路径（目录里的相对路径）。
//! - `platforms`：只在这几个平台上用；不写的哪个平台都行。
//! - `needs`：要的平台能力（`chat::Cap` 的名字）。
//! - `default`：没设过的场所开不开，不写的开。
//!
//! 写错的、认不出的键、认不出的能力整份不收。「挂在哪几处」照核心清单的 `[export] methods`，这里不另写。

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::chat::Cap;

/// 读出来的声明。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// 显示名：语言到名字。
    pub name: BTreeMap<String, String>,
    /// 一句说明：语言到字。
    pub summary: BTreeMap<String, String>,
    /// 图标、主页、仓库、问题反馈、README 的路径：写了的才有。
    pub links: Links,
    /// 只在这几个平台上用；空的是哪个平台都行。
    pub platforms: Vec<String>,
    /// 要的平台能力。
    pub needs: Vec<Cap>,
    /// 没设过的场所开不开。
    pub default: bool,
}

/// 详情页用的几样（施工 I-8 下画）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Links {
    /// 图标。
    pub icon: Option<String>,
    /// 主页。
    pub homepage: Option<String>,
    /// 仓库。
    pub repository: Option<String>,
    /// 问题反馈。
    pub issues: Option<String>,
    /// README 在目录里的相对路径。
    pub readme: Option<String>,
}

/// 文件的样子。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    plugin: Raw,
}

/// `[plugin]` 原样。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    name: BTreeMap<String, String>,
    #[serde(default)]
    summary: BTreeMap<String, String>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    issues: Option<String>,
    #[serde(default)]
    readme: Option<String>,
    #[serde(default)]
    platforms: Vec<String>,
    #[serde(default)]
    needs: Vec<String>,
    #[serde(default = "on")]
    default: bool,
}

/// 不写 `default` 的开。
fn on() -> bool {
    true
}

/// 读 `plugin.toml` 的原文 `text`。
///
/// # Errors
///
/// 不是 TOML、少了 `[plugin]` 或 `name`、键认不出、类型不对、名字是空的、能力认不出：原因（英文，记运行日志、插件页照它说）。
pub fn read(text: &str) -> Result<Declared, String> {
    let file: File = toml::from_str(text).map_err(|error| error.message().to_string())?;
    let raw = file.plugin;
    if raw.name.values().all(|name| name.trim().is_empty()) {
        return Err("plugin.name has no name in any language".to_string());
    }
    let needs = raw
        .needs
        .iter()
        .map(|need| Cap::parse(need).ok_or_else(|| format!("unknown need: {need}")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Declared {
        name: raw.name,
        summary: raw.summary,
        links: Links {
            icon: raw.icon,
            homepage: raw.homepage,
            repository: raw.repository,
            issues: raw.issues,
            readme: raw.readme,
        },
        platforms: raw.platforms,
        needs,
        default: raw.default,
    })
}
