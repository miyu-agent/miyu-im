//! 插件往 im 页面加的内容（施工 I-8 下，设计 01 第四节第 5 条）：首页卡片、会话浮窗的一页、整页，写法照核心的界面块
//! （`widgets.md`）。数据经插件登记的方法取（在这份登录的人的连接上 `package.call`），im 照核心收 `widget.set` 时查的同一份查过、照
//! 语言挑好字再交给页面（`miyu_client::widgets`）。
//!
//! - `plugins.widget {package, place, venue?, page?, language}` → `{"content": [...]}`：
//!   - `place` 是 `home`：调 `chat.home {platform}`，照首页卡片查（最多 8 段）；
//!   - `panel`：调 `chat.panel {platform, venue}`（`venue` 是 `{type, id}`，换成场所编号，例如 `qq:group:777`），照整页查（浮窗里也
//!     能放表单）；
//!   - `page`：调声明里那个整页写的方法 `{platform}`，照整页查。
//! - 插件要声明了那一样（`ui.home`、`ui.panel`、`ui.pages` 里有这个编号），不然 `bad_params`。插件回的 `result` 就是内容（段的数组）。
//! - 写错的拒 `bad_widget_content`，记一行 `WARN plugin widget rejected package=… place=… path=… problem=…`；`package.call` 的错
//!   （`program_not_running`、`method_failed`、`method_timeout`、`unregistered`）照原样交回。

use miyu_chat::VenueKind;
use miyu_client::widgets::{WidgetPlace, check, localize};
use serde_json::{Value, json};

use super::plugins::{bad, core, known};
use super::{Person, Site};
use crate::TARGET;

/// 首页卡片调的方法。
const HOME: &str = "chat.home";

/// 会话浮窗那一页调的方法。
const PANEL: &str = "chat.panel";

/// `plugins.widget`（模块的说明）。
pub(super) async fn widget(
    site: &Site,
    person: &Person,
    params: &Value,
) -> Result<Value, (String, String)> {
    let package = params["package"].as_str().ok_or_else(bad)?;
    let language = params["language"].as_str().unwrap_or("en");
    let plugins = known(person).await?;
    let plugin = plugins
        .iter()
        .find(|plugin| plugin["package"] == package)
        .ok_or_else(bad)?;
    let ui = &plugin["ui"];
    let platform = site.running.first().platform;
    let (method, place, args) = match params["place"].as_str() {
        Some("home") if ui["home"] == true => (
            HOME.to_string(),
            WidgetPlace::Home,
            json!({"platform": platform.name()}),
        ),
        Some("panel") if ui["panel"] == true => {
            let venue = venue_id(platform, &params["venue"]).ok_or_else(bad)?;
            (
                PANEL.to_string(),
                WidgetPlace::Page,
                json!({"platform": platform.name(), "venue": venue}),
            )
        }
        Some("page") => {
            let id = params["page"].as_str().ok_or_else(bad)?;
            let page = ui["pages"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|page| page["id"] == id)
                .ok_or_else(bad)?;
            let method = page["method"].as_str().ok_or_else(bad)?.to_string();
            (
                method,
                WidgetPlace::Page,
                json!({"platform": platform.name()}),
            )
        }
        _ => return Err(bad()),
    };
    // 驱动带的插件在进程里直接问（施工 Q-6）。
    let content = match plugin["builtin"] == true {
        true => site.call_builtin(package, &method, &args).await?,
        false => {
            core(
                person,
                "package.call",
                json!({"package": package, "method": method, "params": args}),
            )
            .await?
        }
    };
    let pages: Vec<&str> = ui["pages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|page| page["id"].as_str())
        .collect();
    if let Err(wrong) = check(place, &pages, &content) {
        tracing::warn!(target: TARGET, package, place = ?place, path = wrong.path, problem = wrong.problem.as_str(), "plugin widget rejected");
        return Err(("bad_widget_content".to_string(), String::new()));
    }
    Ok(json!({"content": localize(&content, language)}))
}

/// 页面交的场所 `{type, id}`（`group`、`private`，号写成字）换成平台 `platform` 上的场所编号；写错的是空的。
pub(super) fn venue_id(platform: crate::chat::Platform, venue: &Value) -> Option<String> {
    let kind = match venue["type"].as_str()? {
        "group" => VenueKind::Group,
        "private" => VenueKind::Private,
        _ => return None,
    };
    let number = venue["id"].as_str()?.parse::<i64>().ok()?;
    Some(platform.venue(kind, number).ok()?.id().to_string())
}

#[cfg(test)]
mod tests;
