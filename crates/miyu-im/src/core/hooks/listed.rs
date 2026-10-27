//! 认得的 im 插件，交给会话浮窗「管理插件」（施工 I-7 中四）和插件页（施工 I-9 上）：每个 `{package, name, summary, default}`（名字、说明
//! 是语言到字的表，页面照语言挑），声明里写了的另带 `icon`、`links`（`homepage`、`repository`、`issues`、`readme`）、`platforms`、
//! `needs`（能力的名字）、`ui`（往 im 的页面加的：`home`、`panel`、`pages` 每个 `{id, title, method}`，施工 I-8 下；都没写的不带）。在这个平台上用不了的另带 `unusable`（`broken`、`other_platforms`、`missing`），缺能力的还带 `missing`（缺的那几样）；
//! 声明读不成的只有编号和 `unusable`。驱动带的（施工 I-10 上）另带 `builtin: true`、`settings`（它的配置项的键的开头，例如
//! `im.qq.identity_`：总开关是 `<开头>on`）。

use serde_json::{Map, Value, json};

use crate::PACKAGE;
use crate::chat::{Cap, Platform};
use crate::plugins::{Plugins, Unusable};

/// 模块的说明：`platform` 是这个平台，`caps` 是驱动的能力表。
pub(super) fn listed(plugins: &Plugins, platform: Platform, caps: &[Cap]) -> Vec<Value> {
    plugins
        .all()
        .map(|(package, declared)| {
            let mut one = match declared {
                Ok(declared) => {
                    let mut one = json!({
                        "package": package,
                        "name": declared.name,
                        "summary": declared.summary,
                        "default": declared.default,
                    });
                    let links = &declared.links;
                    let written: Map<String, Value> = [
                        ("homepage", &links.homepage),
                        ("repository", &links.repository),
                        ("issues", &links.issues),
                        ("readme", &links.readme),
                    ]
                    .into_iter()
                    .filter_map(|(name, link)| Some((name.to_string(), json!(link.as_ref()?))))
                    .collect();
                    if let Some(icon) = &links.icon {
                        one["icon"] = json!(icon);
                    }
                    if !written.is_empty() {
                        one["links"] = Value::Object(written);
                    }
                    if !declared.platforms.is_empty() {
                        one["platforms"] = json!(declared.platforms);
                    }
                    if !declared.venues.is_empty() {
                        one["venues"] = json!(declared.venues);
                    }
                    if !declared.needs.is_empty() {
                        one["needs"] = json!(names(&declared.needs));
                    }
                    let ui = &declared.ui;
                    if ui.home || ui.panel || !ui.pages.is_empty() {
                        one["ui"] = json!({"home": ui.home, "panel": ui.panel, "pages": ui.pages});
                    }
                    one
                }
                Err(_) => json!({"package": package}),
            };
            if let Some(prefix) = plugins.builtin(package) {
                one["builtin"] = json!(true);
                one["settings"] = json!(format!("{PACKAGE}.{}.{prefix}", platform.name()));
            }
            if let Err(why) = plugins.usable(package, platform, caps) {
                one["unusable"] = json!(match &why {
                    Unusable::NotAPlugin | Unusable::Broken(_) => "broken",
                    Unusable::OtherPlatforms => "other_platforms",
                    Unusable::Missing(_) => "missing",
                });
                if let Unusable::Missing(missing) = why {
                    one["missing"] = json!(names(&missing));
                }
            }
            one
        })
        .collect()
}

/// 能力的名字。
fn names(caps: &[Cap]) -> Vec<&'static str> {
    caps.iter().map(|cap| cap.name()).collect()
}

#[cfg(test)]
mod tests;
