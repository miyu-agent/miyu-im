//! 插件的声明 `plugin.toml`（施工 I-8 上，设计 01 第四节第 2 条）：插件带给 im 的那个目录的根上。只有一张 `[plugin]`：
//!
//! - `name`：显示名，照语言写（`zh`、`en`、`ja`），至少一种；`summary`：一句说明，同样照语言写。
//! - `icon`、`homepage`、`repository`、`issues`、`readme`：图标、主页、仓库、问题反馈、README 的路径（目录里的相对路径）。
//! - `platforms`：只在这几个平台上用；不写的哪个平台都行。
//! - `needs`：要的平台能力（`chat::Cap` 的名字）。
//! - `default`：没设过的场所开不开，不写的开。
//! - `[plugin.ui]`（施工 I-8 下，设计 01 第四节第 5 条）：往 im 的页面加什么。`home = true` 放一张首页卡片（im 调它登记的
//!   `chat.home`），`panel = true` 在会话浮窗加一页（`chat.panel`），`[[plugin.ui.pages]]` 一个整页一段：`id`（小写字母开头，小写
//!   字母、数字、`-`，最多 32 个，不重）、`title`（照语言写）、`method`（交内容的方法，写法同核心登记的方法名）；最多 [`PAGES`] 个。
//!
//! 写错的、认不出的键、认不出的能力整份不收。「挂在哪几处」照核心清单的 `[export] methods`，这里不另写。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
    /// 往 im 的页面加的（施工 I-8 下）。
    pub ui: Ui,
}

/// 往 im 的页面加的（模块的说明）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ui {
    /// 放一张首页卡片。
    pub home: bool,
    /// 在会话浮窗加一页。
    pub panel: bool,
    /// 整页，照写的先后。
    pub pages: Vec<Page>,
}

/// 一个整页。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    /// 编号。
    pub id: String,
    /// 标题：语言到字。
    pub title: BTreeMap<String, String>,
    /// 交内容的方法。
    pub method: String,
}

/// 一个插件最多几个整页。
pub const PAGES: usize = 8;

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
    #[serde(default)]
    ui: RawUi,
}

/// `[plugin.ui]` 原样。
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUi {
    #[serde(default)]
    home: bool,
    #[serde(default)]
    panel: bool,
    #[serde(default)]
    pages: Vec<Page>,
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
        ui: ui(raw.ui)?,
    })
}

/// 查 `[plugin.ui]`：整页的编号、方法名的写法，编号不重，标题不空，最多 [`PAGES`] 个。
fn ui(raw: RawUi) -> Result<Ui, String> {
    if raw.pages.len() > PAGES {
        return Err(format!("plugin.ui.pages has more than {PAGES} pages"));
    }
    for (at, page) in raw.pages.iter().enumerate() {
        if !page_id(&page.id) {
            return Err(format!("plugin.ui.pages: bad id {:?}", page.id));
        }
        if raw.pages[..at].iter().any(|before| before.id == page.id) {
            return Err(format!("plugin.ui.pages: id {} twice", page.id));
        }
        if !method_name(&page.method) {
            return Err(format!(
                "plugin.ui.pages.{}: bad method {:?}",
                page.id, page.method
            ));
        }
        if page.title.values().all(|title| title.trim().is_empty()) {
            return Err(format!("plugin.ui.pages.{}: no title", page.id));
        }
    }
    Ok(Ui {
        home: raw.home,
        panel: raw.panel,
        pages: raw.pages,
    })
}

/// 整页的编号：小写字母开头，小写字母、数字、`-`，最多 32 个（同核心界面块的编号）。
fn page_id(id: &str) -> bool {
    id.len() <= 32
        && id.starts_with(|c: char| c.is_ascii_lowercase())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// 方法名：小写字母开头，小写字母、数字、`_`、`.`、`-`，1 到 64 个（同核心 `package.methods`）。
fn method_name(name: &str) -> bool {
    name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-'))
}
