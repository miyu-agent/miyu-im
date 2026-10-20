//! 页面调群管理（施工 Q-6，`miyu_im::plugins::Builtin::call`）：
//!
//! - `chat.panel {platform, venue}`：会话浮窗的「群管理」页（`page::panel`）；不是群的回 `bad_params`。
//! - `admin.page {platform}`：面板，她在的每个群一项（`page::section`）：群照第一个连着的号问 QQ，她的身份一个个问；没连着的回
//!   一段字。
//! - `admin.spam {group, values}`：存这个群的刷屏处理（只交改过的格：`on`、`window` 秒、`mute` 分钟），写错的回 `problems`。
//! - `admin.view {group, values}`：存面板上这个群怎么看（`action`、`period`）。
//! - `admin.approval {group, values}`：存这个群的入群审批的通过条件（`rule`，字，`null` 是清掉），施工 Q-7。
//! - `admin.clear {group}`：头一次只记下点了，一分钟以内再点一次才删这个群的记录。

use serde_json::{Map, Value, json};

use miyu_im::plugins::Context;

use super::groups::{self, Group};
use super::page::{self, t};
use super::records;
use crate::admin;

/// 调群管理的方法（模块的说明）。
pub(crate) async fn call(context: &Context, method: &str, params: &Value) -> Result<Value, String> {
    let bad = || "bad_params".to_string();
    let now = records::now();
    let summary = admin::summarizing(&context.config);
    match method {
        "chat.panel" => {
            let group =
                venue_group(params["venue"].as_str().unwrap_or_default()).ok_or_else(bad)?;
            let settings = groups::read(&context.store)
                .remove(&group.to_string())
                .unwrap_or_default();
            let records = records::read(&context.store, group);
            Ok(page::panel(group, &settings, &records, summary, now))
        }
        "admin.page" => Ok(whole(context, summary, now).await),
        "admin.spam" | "admin.view" | "admin.clear" | "admin.approval" => {
            let group = params["group"]
                .as_i64()
                .filter(|group| *group > 0)
                .ok_or_else(bad)?;
            let values = params["values"].as_object().cloned().unwrap_or_default();
            saved(context, method, group, &values, now)
        }
        _ => Err("unknown_method".to_string()),
    }
}

/// 场所编号 `qq:group:777` 里的群号；别的是空的。
fn venue_group(venue: &str) -> Option<i64> {
    venue
        .strip_prefix("qq:group:")?
        .parse()
        .ok()
        .filter(|group: &i64| *group > 0)
}

/// 面板：她在的每个群一项。
async fn whole(context: &Context, summary: bool, now: u64) -> Value {
    let Some((bot, link)) = context.bots.first() else {
        return json!([{"type": "text", "text": t("QQ 没连着：连上以后这里列出她在的群。", "QQ is not connected: the groups appear once it is.", "QQ が接続されていません。接続するとグループが表示されます。"), "tone": "dim"}]);
    };
    let listed = link.groups().await.unwrap_or_default();
    let all = groups::read(&context.store);
    let mut items = Vec::new();
    for group in listed.iter().take(page::SHOWN) {
        let admin = admin::admin_here(link.as_ref(), group.id, bot).await.ok();
        let settings = all.get(&group.id.to_string()).cloned().unwrap_or_default();
        let records = records::read(&context.store, group.id);
        items.push(page::section(
            (group.id, group.name.as_deref()),
            admin,
            (&settings, &records),
            (summary, now),
        ));
    }
    if items.is_empty() {
        return json!([{"type": "text", "text": t("她还不在任何群里。", "She is not in any group yet.", "まだどのグループにも入っていません。"), "tone": "dim"}]);
    }
    json!([{"type": "sections", "items": items}])
}

/// 存表单、点按钮（模块的说明）：表单回 `{"saved": …}`，按钮回 `{}`。
fn saved(
    context: &Context,
    method: &str,
    group: i64,
    values: &Map<String, Value>,
    now: u64,
) -> Result<Value, String> {
    let mut problems = Vec::new();
    let mut check = |key: &str, ok: bool| {
        if !ok {
            problems.push(json!({"key": key, "message": t("这一格写得不对。", "This value is not valid.", "この値は正しくありません。")}));
        }
    };
    let number = |key: &str, min: u64, max: u64| {
        values
            .get(key)
            .map(|value| value.as_u64().filter(|n| (min..=max).contains(n)))
    };
    let change: Box<dyn FnOnce(&mut Group)> = match method {
        "admin.spam" => {
            let on = values.get("on").map(Value::as_bool);
            let window = number("window", 10, 3600);
            let mute = number("mute", 1, 43_200);
            check("on", !matches!(on, Some(None)));
            check("window", !matches!(window, Some(None)));
            check("mute", !matches!(mute, Some(None)));
            Box::new(move |group: &mut Group| {
                if let Some(Some(on)) = on {
                    group.spam.on = on;
                }
                if let Some(Some(window)) = window {
                    group.spam.window = window;
                }
                if let Some(Some(mute)) = mute {
                    group.spam.mute = mute * 60;
                }
            })
        }
        "admin.approval" => {
            // 通过条件：字，最多 2000 个；`null` 是清掉（不审）。
            let rule = values.get("rule").map(|value| match value {
                Value::Null => Some(String::new()),
                value => value
                    .as_str()
                    .filter(|text| text.chars().count() <= 2000)
                    .map(str::to_string),
            });
            check("rule", !matches!(rule, Some(None)));
            Box::new(move |group: &mut Group| {
                if let Some(Some(rule)) = rule {
                    group.approval.rule = rule;
                }
            })
        }
        "admin.view" => {
            let pick = |key: &str, allowed: &[&str]| {
                values.get(key).map(|value| {
                    value
                        .as_str()
                        .filter(|text| allowed.contains(text))
                        .map(str::to_string)
                })
            };
            let action = pick("action", &["all", "mute", "unmute", "kick", "join"]);
            let period = pick("period", &["week", "month", "all"]);
            check("action", !matches!(action, Some(None)));
            check("period", !matches!(period, Some(None)));
            Box::new(move |group: &mut Group| {
                if let Some(Some(action)) = action {
                    group.view.action = action;
                }
                if let Some(Some(period)) = period {
                    group.view.period = period;
                }
            })
        }
        _ => {
            let asked = groups::read(&context.store)
                .get(&group.to_string())
                .and_then(|settings| settings.clearing)
                .is_some_and(|at| now < at.saturating_add(60));
            if asked {
                if let Err(error) = std::fs::remove_file(records::file(&context.store, group))
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    return Err("method_failed".to_string());
                }
                tracing::info!(target: miyu_im::TARGET, group, "moderation records cleared from the page");
            }
            groups::update(&context.store, group, |settings| {
                settings.clearing = (!asked).then_some(now);
            })
            .map_err(|_| "method_failed".to_string())?;
            return Ok(json!({}));
        }
    };
    if !problems.is_empty() {
        return Ok(json!({"saved": false, "problems": problems}));
    }
    groups::update(&context.store, group, change).map_err(|_| "method_failed".to_string())?;
    Ok(json!({"saved": true}))
}

#[cfg(test)]
mod tests;
