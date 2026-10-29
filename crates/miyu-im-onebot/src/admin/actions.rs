//! 页面调群管理（施工 Q-6，`miyu_im::plugins::Builtin::call`）：
//!
//! - `chat.panel {platform, venue}`：会话浮窗的「群管理」页（`page::panel`）；不是群的回 `bad_params`。
//! - `admin.page {platform}`：面板，她在的每个群一项（`page::section`）：群照每个连着的号问 QQ，她的身份并着问（施工 I-17）；没连
//!   着的回一段字。
//! - `admin.spam {group, values}`：存这个群的刷屏处理（只交改过的格：`on`、`window` 秒、`mute` 分钟），写错的回 `problems`。
//! - `admin.view {group, values}`：存面板上这个群怎么看（`action`、`period`）。
//! - `admin.approval {group, values}`：存这个群的入群审批的通过条件（`rule`，字，`null` 是清掉），施工 Q-7。
//! - `admin.clear {group}`：头一次只记下点了，一分钟以内再点一次才删这个群的记录。

use std::sync::Arc;

use serde_json::{Map, Value, json};

use miyu_im::chat::{Id, Link};
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
            let records = records::read(&context.store, group.clone());
            Ok(page::panel(group, &settings, &records, summary, now))
        }
        "admin.page" => Ok(whole(context, summary, now).await),
        "admin.spam" | "admin.view" | "admin.clear" | "admin.approval" => {
            let group = params["group"]
                .as_i64()
                .filter(|group| *group > 0)
                .map(Id::from)
                .ok_or_else(bad)?;
            let values = params["values"].as_object().cloned().unwrap_or_default();
            saved(context, method, group, &values, now)
        }
        _ => Err("unknown_method".to_string()),
    }
}

/// 场所编号里的群号（`qq:30003:group:777`，施工 I-81 起带机器人号；老写法 `qq:group:777` 也认）：照群聊内核解，只认 QQ 的群、号是
/// 正整数；别的是空的。
fn venue_group(venue: &str) -> Option<Id> {
    let id = miyu_client::ids::VenueId::parse(venue).ok()?;
    let venue = miyu_chat::Venue::parse(&id)?;
    if venue.platform() != crate::onebot::PLATFORM.name()
        || venue.kind() != miyu_chat::VenueKind::Group
    {
        return None;
    }
    venue
        .number()
        .parse()
        .ok()
        .filter(|group: &i64| *group > 0)
        .map(Id::from)
}

/// 面板：她在的每个群一项（施工 I-17：连着的号都看，一个群照在里面的号问身份，都并着问，以前只看第一个号、一个个等）。
async fn whole(context: &Context, summary: bool, now: u64) -> Value {
    let bots = context.bots.all();
    if bots.is_empty() {
        return json!([{"type": "text", "text": t("QQ 没连着：连上以后这里列出她在的群。", "QQ is not connected: the groups appear once it is.", "QQ が接続されていません。接続するとグループが表示されます。"), "tone": "dim"}]);
    }
    // 每个号在的群，并着问；群的先后照号的先后、QQ 列的先后（同以前）。
    let mut listing = tokio::task::JoinSet::new();
    for (index, (bot, link)) in bots.into_iter().enumerate() {
        listing.spawn(async move { (index, bot, link.groups().await.unwrap_or_default(), link) });
    }
    let mut answered = Vec::new();
    while let Some(Ok(one)) = listing.join_next().await {
        answered.push(one);
    }
    answered.sort_by_key(|(index, ..)| *index);
    let mut listed: Vec<Listed> = Vec::new();
    for (_, bot, groups, link) in answered {
        for group in groups {
            match listed.iter_mut().find(|(id, _, _)| *id == group.id) {
                Some((_, _, inside)) => inside.push((bot.clone(), Arc::clone(&link))),
                None => listed.push((group.id, group.name, vec![(bot.clone(), Arc::clone(&link))])),
            }
        }
    }
    listed.truncate(page::SHOWN);
    // 每个群里每个号的身份，并着问：有一个是管理员就算；都问到了、都不是的不是；别的问不到。
    let mut asking = tokio::task::JoinSet::new();
    for (group, _, inside) in &listed {
        for (bot, link) in inside {
            let (group, bot, link) = (group.clone(), bot.clone(), Arc::clone(link));
            asking.spawn(async move {
                (
                    group.clone(),
                    admin::admin_here(link.as_ref(), &group, &bot).await.ok(),
                )
            });
        }
    }
    let mut ranks: Vec<(Id, Option<bool>)> = Vec::new();
    while let Some(Ok(asked)) = asking.join_next().await {
        ranks.push(asked);
    }
    let all = groups::read(&context.store);
    let items: Vec<Value> = listed
        .iter()
        .map(|(group, name, _)| {
            let admin = role(
                ranks
                    .iter()
                    .filter(|(id, _)| id == group)
                    .map(|(_, admin)| *admin),
            );
            let settings = all.get(&group.to_string()).cloned().unwrap_or_default();
            let records = records::read(&context.store, group.clone());
            page::section(
                (group.clone(), name.as_deref()),
                admin,
                (&settings, &records),
                (summary, now),
            )
        })
        .collect();
    if items.is_empty() {
        return json!([{"type": "text", "text": t("她还不在任何群里。", "She is not in any group yet.", "まだどのグループにも入っていません。"), "tone": "dim"}]);
    }
    json!([{"type": "sections", "items": items}])
}

/// 面板上的一个群：群号、群名、在里面的号和它的连接。
type Listed = (Id, Option<String>, Vec<(Id, Arc<dyn Link>)>);

/// 一个群里几个号问到的身份（纯逻辑）：有一个是管理员就是；都问到了、都不是的不是；别的（有问不到的、一个都没有）是空的。
pub(crate) fn role(asked: impl IntoIterator<Item = Option<bool>>) -> Option<bool> {
    let mut all_no = true;
    let mut any = false;
    for one in asked {
        any = true;
        match one {
            Some(true) => return Some(true),
            Some(false) => {}
            None => all_no = false,
        }
    }
    (any && all_no).then_some(false)
}

/// 存表单、点按钮（模块的说明）：表单回 `{"saved": …}`，按钮回 `{}`。
fn saved(
    context: &Context,
    method: &str,
    group: Id,
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
                if let Err(error) =
                    std::fs::remove_file(records::file(&context.store, group.clone()))
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    return Err("method_failed".to_string());
                }
                tracing::info!(target: miyu_im::TARGET, group = %group, "moderation records cleared from the page");
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
