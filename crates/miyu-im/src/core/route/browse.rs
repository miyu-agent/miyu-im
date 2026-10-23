//! 后台「平台会话」页要的三个方法（施工 I-34 上，项目主人 2026-10-23 定：场所会话只读，看的地方放进 im 自己的后台页，一个「平台
//! 会话」页按连接分组列会话，点进去看只读的记录；管理员能删）。核心 Y-2 上起人碰不到场所会话，这几个都在桥自己的连接上问核心
//! （场所会话的属主就是桥的系统账号）：
//!
//! - `sessions.list` → `{connections: [{platform, bot, sessions: [{session, venue, kind, id, name}]}]}`：照核心的
//!   `venue.sessions`（从新到旧），桥认得是哪个机器人号收进来的照它分组，认不得的（还没来过消息）放进 `bot` 是空的那一组；
//!   名字照 NapCat 问到的群名、好友名（同会话页的 `cards.get`），问不到的是空的。
//! - `sessions.page {session, before?}` → 核心 `view.page {view: true}` 的回答原样，`judged` 多一张表：条目编号（`m<序号>`）
//!   → 那一条的判断 `{outcome, why, reason}`（照同一页的原始事件里的 `ext.im.chat.decided`）。只认桥自己的会话。
//! - `sessions.delete {session}` → 核心 `session.delete` 的回答：只删桥自己的会话。
//!
//! 不认得的会话拒 `session_not_found`，参数写错的拒 `bad_params`；核心拒了的照它的原因拒。

use std::collections::HashMap;

use serde_json::{Map, Value, json};

use super::Route;
use super::cards::lists;
use crate::chat::{Bots, To};
use crate::core::Answerer;
use crate::core::caller::Caller;
use crate::core::methods::{
    SESSIONS_DELETE as DELETE, SESSIONS_LIST as LIST, SESSIONS_PAGE as PAGE,
};
use crate::core::reason;

/// 一页最多几轮（核心 `view.page` 的 `turns`）。
const TURNS: u64 = 20;

/// 桥认得的一个场所会话：收进它的机器人号、对方。
#[derive(Clone)]
struct Known {
    bot: i64,
    to: To,
}

impl Route {
    /// 核心转来的一次 `method.call`（`pushed`）是不是这三个：是的照答，交回真。
    pub(super) fn browse_called(&mut self, pushed: &Value) -> bool {
        let method = pushed["params"]["method"].as_str().unwrap_or_default();
        if ![LIST, PAGE, DELETE].contains(&method) {
            return false;
        }
        let (id, args) = (pushed["id"].clone(), &pushed["params"]["params"]);
        let (caller, answerer) = (self.core.caller(), self.core.answerer());
        let known: HashMap<String, Known> = self
            .peers
            .iter()
            .map(|(session, peer)| {
                let known = Known {
                    bot: peer.bot,
                    to: peer.to,
                };
                (session.clone(), known)
            })
            .collect();
        if method == LIST {
            let bots = std::sync::Arc::clone(&self.bots);
            let platform = self.platform.name().to_string();
            self.chores
                .spawn(listed(id, (known, bots, platform), caller, answerer));
            return true;
        }
        let Some(session) = args["session"].as_str().map(str::to_string) else {
            answerer.refuse(&id, "bad_params");
            return true;
        };
        if !known.contains_key(&session) {
            answerer.refuse(&id, "session_not_found");
            return true;
        }
        if method == PAGE {
            let before = args["before"].as_u64();
            self.chores
                .spawn(paged(id, (session, before), caller, answerer));
        } else {
            self.chores.spawn(deleted(id, session, caller, answerer));
        }
        true
    }
}

/// `sessions.list`（模块的说明）。
async fn listed(
    id: Value,
    (known, bots, platform): (HashMap<String, Known>, std::sync::Arc<Bots>, String),
    caller: Caller,
    answerer: Answerer,
) {
    let Ok(reply) = caller.call("venue.sessions", json!({})).await else {
        return;
    };
    if let Some(why) = reason(&reply) {
        answerer.refuse(&id, why);
        return;
    }
    let names = lists(&bots, &caller).await;
    let named = |rows: &[Value]| -> HashMap<i64, String> {
        rows.iter()
            .filter_map(|row| Some((row["id"].as_i64()?, row["name"].as_str()?.to_string())))
            .collect()
    };
    let (groups, friends) = (named(&names.groups), named(&names.friends));
    let sessions = reply["result"]["sessions"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    answerer.answer(
        &id,
        grouped(&sessions, &known, (&groups, &friends), &platform),
    );
}

/// 照桥认得的机器人号把 `venue.sessions` 的 `sessions` 分组（纯逻辑）：组照第一次出现的先后，组里照原来的先后（从新到旧）。
fn grouped(
    sessions: &[Value],
    known: &HashMap<String, Known>,
    (groups, friends): (&HashMap<i64, String>, &HashMap<i64, String>),
    platform: &str,
) -> Value {
    let mut order: Vec<Option<i64>> = Vec::new();
    let mut rows: HashMap<Option<i64>, Vec<Value>> = HashMap::new();
    for one in sessions {
        let Some(session) = one["session"].as_str() else {
            continue;
        };
        let peer = known.get(session);
        let bot = peer.map(|peer| peer.bot);
        let (kind, number, name) = match peer.map(|peer| peer.to) {
            Some(To::Group(group)) => ("group", Some(group), groups.get(&group)),
            Some(To::Private(user)) => ("private", Some(user), friends.get(&user)),
            None => ("unknown", None, None),
        };
        if !rows.contains_key(&bot) {
            order.push(bot);
        }
        rows.entry(bot).or_default().push(json!({
            "session": session,
            "venue": one["venue"],
            "kind": kind,
            "id": number,
            "name": name,
        }));
    }
    let connections: Vec<Value> = order
        .into_iter()
        .map(|bot| {
            let sessions = rows.remove(&bot).unwrap_or_default();
            json!({"platform": platform, "bot": bot, "sessions": sessions})
        })
        .collect();
    json!({"connections": connections})
}

/// `sessions.page`（模块的说明）。
async fn paged(
    id: Value,
    (session, before): (String, Option<u64>),
    caller: Caller,
    answerer: Answerer,
) {
    let mut params = json!({"session": session, "turns": TURNS, "view": true});
    if let Some(before) = before {
        params["before"] = json!(before);
    }
    let Ok(viewed) = caller.call("view.page", params.clone()).await else {
        return;
    };
    if let Some(why) = reason(&viewed) {
        answerer.refuse(&id, why);
        return;
    }
    params["view"] = json!(false);
    let judged = match caller.call("view.page", params).await {
        Ok(raw) if reason(&raw).is_none() => judged(&raw["result"]["events"]),
        _ => Map::new(),
    };
    let mut answer = viewed["result"].clone();
    answer["judged"] = Value::Object(judged);
    answerer.answer(&id, answer);
}

/// 一页原始事件 `events` 里的判断（纯逻辑）：条目编号 `m<序号>` → `{outcome, why, reason}`。一笔判断管几条的，每条都记；同一条
/// 判过几次的照后一次。
fn judged(events: &Value) -> Map<String, Value> {
    let mut judged = Map::new();
    for event in events.as_array().into_iter().flatten() {
        if event["kind"] != "ext.im.chat.decided" {
            continue;
        }
        let body = &event["body"];
        let why = body
            .get("why")
            .filter(|why| why.is_string())
            .or_else(|| {
                body.get("inbound")
                    .filter(|inbound| inbound != &&json!("pass"))
            })
            .cloned()
            .unwrap_or(Value::Null);
        let note = json!({
            "outcome": body["outcome"],
            "why": why,
            "reason": body["judge"]["reason"],
        });
        for seq in body["msgs"].as_array().into_iter().flatten() {
            if let Some(seq) = seq.as_u64() {
                judged.insert(format!("m{seq}"), note.clone());
            }
        }
    }
    judged
}

/// `sessions.delete`（模块的说明）。
async fn deleted(id: Value, session: String, caller: Caller, answerer: Answerer) {
    let Ok(reply) = caller
        .call("session.delete", json!({"session": session}))
        .await
    else {
        return;
    };
    match reason(&reply) {
        Some(why) => answerer.refuse(&id, why),
        None => answerer.answer(&id, json!({"deleted": true})),
    }
}

#[cfg(test)]
mod tests;
