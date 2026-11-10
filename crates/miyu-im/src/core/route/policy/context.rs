//! 会话的上下文上限（施工 I-189，核心 CTX-1）：卡片、场所规则的 `context` 经 `session.configure {session, context}` 设到场所会话上，
//! 清掉的交 `null`（回到照模型的窗口）。
//!
//! - 单独发一次，不和模型、人格、预设拼在一起：核心一格不收就什么都不换，上限设小了不能连累换模型。
//! - 设得太小的核心回 `bad_params`（`data.field` 是 `context`，`data.min` 是最少要多少）：照会话记下最少多少（[`Refused`]），会话页
//!   问卡片时交出去，那一格下面写「最少 x」；后来设成了、清掉了的拿掉。
//! - 不碰回合：核心当场换，有回合在跑也收。

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use miyu_chat::{Venue, VenueKind};
use miyu_client::ids::VenueId;
use serde_json::{Value, json};

use crate::TARGET;
use crate::core::route::Route;
use crate::core::{Caller, reason};

/// 设不上的：会话编号 → 核心说最少要多少。跟核心的那一头和发的小任务共用一份。
#[derive(Debug, Clone, Default)]
pub(in crate::core::route) struct Refused(Arc<Mutex<HashMap<String, u64>>>);

impl Refused {
    /// 这一刻设不上的几个：会话编号、最少多少。
    pub(in crate::core::route) fn all(&self) -> Vec<(String, u64)> {
        let held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        held.iter()
            .map(|(session, min)| (session.clone(), *min))
            .collect()
    }

    /// 会话 `session` 设不上：最少 `min`；空的是设成了、清掉了。
    fn mark(&self, session: &str, min: Option<u64>) {
        let mut held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match min {
            Some(min) => held.insert(session.to_string(), min),
            None => held.remove(session),
        };
    }
}

/// 给会话 `session` 设上限 `context`（token 数，`null` 是去掉），结果记进 `refused`。核心拒了别的记一行运行日志；断了的不管。
pub(in crate::core::route) async fn pin(
    caller: &Caller,
    session: &str,
    context: Value,
    refused: &Refused,
) {
    let params = json!({"session": session, "context": context});
    let Ok(reply) = caller.call("session.configure", params).await else {
        return;
    };
    match reason(&reply) {
        None => {
            tracing::info!(target: TARGET, session, context = %context, "venue context pinned");
            refused.mark(session, None);
        }
        Some(why) => {
            let data = &reply["error"]["data"];
            let min = (data["field"] == "context")
                .then(|| data["min"].as_u64())
                .flatten();
            tracing::warn!(target: TARGET, session, context = %context, reason = why, min, "venue context not pinned");
            if min.is_some() {
                refused.mark(session, min);
            }
        }
    }
}

impl Route {
    /// 会话页问卡片时交的「最少多少」（`cards.get` 的 `context_min`）：`{"group": {号: 最少}, "private": {号: 最少}}`，只放平台
    /// `platform` 的。同一个号几个机器人的会话都设不上的，交最大的那个。
    pub(in crate::core::route) fn context_min(&self, platform: &str) -> Value {
        let mut found = json!({"group": {}, "private": {}});
        for (session, min) in self.context_refused.all() {
            let Some(venue) = self
                .venues
                .iter()
                .find(|(_, of)| **of == session)
                .and_then(|(venue, _)| VenueId::parse(venue).ok())
                .and_then(|id| Venue::parse(&id))
                .filter(|venue| venue.platform() == platform)
            else {
                continue;
            };
            let kind = match venue.kind() {
                VenueKind::Group => "group",
                VenueKind::Private => "private",
            };
            let held = &mut found[kind][venue.number()];
            if held.as_u64().is_none_or(|was| was < min) {
                *held = json!(min);
            }
        }
        found
    }
}
