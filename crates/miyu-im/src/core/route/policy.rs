//! 场所会话的模型、人格、预设（施工 O-58 补三起模型，补四加人格、预设；设计 01 配置项「对话模型」「AI 人格」）：卡片、场所规则的
//! `model`、`persona`、`preset` 经核心的 `session.configure` 换到已有的场所会话上（核心第 5、6 件：`model: null` 回到跟着配置的
//! `models.chat`；换人格、预设先压缩一轮，下一轮换上）。
//!
//! - 第一次找到会话（`session`）：规则设了的几样都发一次，等它回了再交消息（新造的会话 `venue.session` 已经照规则带了人格、预设，
//!   一样的核心什么都不记；找回的已有会话照这时的规则对上，桥停着时改了卡片的也跟上）。没设的不碰：人在终端、网页里换的照旧。
//! - 卡片存了（`cards.save`、两页改一张卡）：记着的每个场所会话比一比换卡以前、以后规则设的三样，变了的发：模型清掉的交 `null`
//!   回到跟着配置；人格、预设清掉的不发（核心的 `persona: null` 是无人格，不是「回到默认」，人要换回去在卡上选一个）。另起小任务，
//!   不等。
//! - 有回合在跑（`turn_running`）：隔 [`RETRY`] 再发，最多 [`TRIES`] 次。核心拒了别的（`unknown_model`、`unknown_persona` 这类）
//!   记一行运行日志，会话照原来的。

use std::collections::HashMap;
use std::time::Duration;

use miyu_chat::Venue;
use miyu_client::ids::VenueId;
use serde_json::{Map, Value, json};

use super::Route;
use super::applied;
use crate::TARGET;
use crate::core::{Caller, reason};
use crate::rules::Applied;

#[cfg(test)]
mod tests;

/// 有回合在跑时隔多久再发。
const RETRY: Duration = Duration::from_secs(10);

/// 有回合在跑时最多发几次。
const TRIES: u32 = 30;

/// 规则设到一个场所上的三样：没设的是空的。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Wanted {
    model: Option<String>,
    persona: Option<String>,
    preset: Option<String>,
}

impl Wanted {
    /// 套到场所上的规则 `applied` 设的三样。
    pub(super) fn of(applied: &Applied) -> Wanted {
        Wanted {
            model: applied::model(applied).map(str::to_string),
            persona: applied::text_of(applied, "persona").map(str::to_string),
            preset: applied::text_of(applied, "preset").map(str::to_string),
        }
    }

    /// 第一次找到会话时发的：设了的几样。都没设的是空的。
    pub(super) fn first(&self) -> Option<Map<String, Value>> {
        let mut fields = Map::new();
        for (key, value) in [
            ("model", &self.model),
            ("persona", &self.persona),
            ("preset", &self.preset),
        ] {
            if let Some(value) = value {
                fields.insert(key.into(), json!(value));
            }
        }
        (!fields.is_empty()).then_some(fields)
    }

    /// 从 `before` 换成这个要发的：模型变了的（清掉的是 `null`），人格、预设变成另一个的。没有要发的是空的。
    fn since(&self, before: &Wanted) -> Option<Map<String, Value>> {
        let mut fields = Map::new();
        if self.model != before.model {
            fields.insert("model".into(), json!(self.model));
        }
        for (key, now, was) in [
            ("persona", &self.persona, &before.persona),
            ("preset", &self.preset, &before.preset),
        ] {
            if let Some(now) = now.as_ref().filter(|now| Some(*now) != was.as_ref()) {
                fields.insert(key.into(), json!(now));
            }
        }
        (!fields.is_empty()).then_some(fields)
    }
}

impl Route {
    /// 记着的场所会话此刻照规则该是的三样：会话编号 → 三样。记着的私聊都是套了规则的（白名单成员、照「其他私聊」接的陌生人）
    /// 或者终端管理员（卡片管不到，换卡以前以后一样，不会发）。
    pub(super) fn wanted(&mut self) -> HashMap<String, Wanted> {
        let venues: Vec<(String, String)> = self
            .venues
            .iter()
            .map(|(venue, session)| (venue.clone(), session.clone()))
            .collect();
        let mut wanted = HashMap::new();
        for (venue, session) in venues {
            let Some(parsed) = VenueId::parse(&venue).ok().and_then(|id| Venue::parse(&id)) else {
                continue;
            };
            let applied = self.applied(&parsed);
            wanted.insert(session, Wanted::of(&applied));
        }
        wanted
    }

    /// 卡片换了以后（`before` 是换以前 [`Route::wanted`] 的样子）：变了的发，另起小任务，不等。换了人格的先记给判官那边。
    pub(super) fn repin(&mut self, before: &HashMap<String, Wanted>) {
        for (session, now) in self.wanted() {
            let Some(fields) = now.since(&before.get(&session).cloned().unwrap_or_default()) else {
                continue;
            };
            if let Some(persona) = fields.get("persona").and_then(Value::as_str) {
                self.judges.subscribed(&session, Some(persona));
            }
            let caller = self.core.caller();
            self.chores.spawn(async move {
                pin(&caller, &session, &fields, TRIES).await;
            });
        }
    }

    /// 第一次找到会话 `session`：规则设了的几样 `wanted` 发一次，等它回了。有回合在跑的另起小任务接着试。
    pub(super) async fn pin_found(&mut self, session: &str, wanted: &Wanted) {
        let Some(fields) = wanted.first() else {
            return;
        };
        if let Some(persona) = wanted.persona.as_deref() {
            self.judges.subscribed(session, Some(persona));
        }
        let caller = self.core.caller();
        if !pin(&caller, session, &fields, 1).await {
            let session = session.to_string();
            self.chores.spawn(async move {
                tokio::time::sleep(RETRY).await;
                pin(&caller, &session, &fields, TRIES).await;
            });
        }
    }
}

/// 经 `session.configure` 给会话 `session` 发 `fields`，有回合在跑的隔 [`RETRY`] 再发，一共最多 `tries` 次。交回是不是发完了
/// （假的是一直有回合在跑，调的一方另起任务接着试）。核心拒了别的记一行运行日志；核心断了的不管（跟核心的那一头自己会停）。
async fn pin(caller: &Caller, session: &str, fields: &Map<String, Value>, tries: u32) -> bool {
    let mut params = fields.clone();
    params.insert("session".into(), json!(session));
    let shown = Value::Object(fields.clone()).to_string();
    for tried in 1..=tries {
        let Ok(reply) = caller
            .call("session.configure", Value::Object(params.clone()))
            .await
        else {
            return true;
        };
        match reason(&reply) {
            None => {
                tracing::info!(target: TARGET, session, fields = %shown, "venue policy pinned");
                return true;
            }
            Some("turn_running") if tried < tries => tokio::time::sleep(RETRY).await,
            Some("turn_running") => {
                tracing::info!(target: TARGET, session, fields = %shown, "venue policy waits, turn running");
                return false;
            }
            Some(reason) => {
                tracing::warn!(target: TARGET, session, fields = %shown, reason, "venue policy not pinned");
                return true;
            }
        }
    }
    true
}
