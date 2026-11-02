//! 时不时看一眼的接线（设计 05 第二节，施工 I-89；纯逻辑在 `crate::glance`）：照场所规则的 `glance` 给名单里的群、私聊排核心的一次性定时
//! （`schedule.add`，`target` 是 `{"package": {}}`），到点核心推 `schedule.fired`，桥先过一道再决定叫不叫她。
//!
//! - 对账（[`Route::reconcile_glances`]）：起来订阅完、卡片存了以后列自己建的定时，照开关删多的、补没有的（核心停着、桥没连着时
//!   到点的一次性定时核心直接删掉，靠这里补回来）。头一回来消息的群另外补一个（[`Route::glance_found`]）。
//! - 到点过一道（[`Route::glance_due`]）：开关关了的什么都不做；睡着的顺延到醒；这个群正有一轮在跑的等那一轮结束再过一遍；
//!   上次看过以后没有新消息的不请求模型，照上一次的间隔再排；都不是的照新消息开一轮（`session.respond` 的 `to`），带一块
//!   `glance` 事实。私聊（I-89 下，项目主人 11-02 定）不看有没有新消息：闲着就开一轮（`to` 是空的，只带那块事实，核心 O-64），
//!   让她决定要不要主动开口；正有一轮在跑的同样等它结束。
//! - 看一眼那一轮结束了（[`Route::glance_turn_ended`]）：没定下一次的照 `glance_seconds`（出厂 25 分钟）再排。她用工具定了的
//!   （[`Route::glance_after`]，I-89 下）照她的。

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use miyu_chat::{Venue, VenueKind};
use miyu_client::ids::VenueId;
use miyu_client::time::Timestamp;
use serde_json::{Value, json};

use super::judges::Tag;
use super::lanes::Responded;
use super::{Route, applied};
use crate::TARGET;
use crate::core::{Gone, reason};
use crate::glance::{Prompt, reconcile, unanswered};

/// 看一眼那一块事实的 `kind`。
const GLANCE: &str = "glance";

/// 一秒的毫秒数。
const SECOND: i64 = 1000;

/// 桥记着的看一眼：核心那边的定时是准的，这里只为不重复建、认得出哪一轮是看一眼开的。
#[derive(Debug, Default)]
pub(super) struct Glances {
    /// 没定下一次时隔多久（`bridge.json` 的 `glance_seconds`）。
    default: Duration,
    /// 对过账了：起来时、卡片存了以后是假的，下一次 [`Route::sync_glances`] 照它对账。
    synced: bool,
    /// 场所编号 → 等着的定时编号。
    armed: HashMap<String, String>,
    /// 会话编号 → 到点时正有一轮在跑、等它结束再过一遍的那一次（定时编号、交回的那一份）。
    postponed: HashMap<String, (String, Prompt)>,
    /// 会话编号 → 看一眼开的那一轮（场所编号、看到哪一条）：结束了照 [`Glances::default`] 再排。
    looking: HashMap<String, (String, u64)>,
    /// 私聊的场所编号 → 会话编号（I-89 下）：起来时列出来的、找到过的都记着，换白名单、存卡片时不忘（`forget_privates` 只管下一条
    /// 消息怎么找会话）。
    privates: HashMap<String, String>,
    /// 正有一轮在跑的私聊会话（I-89 下）：私聊没有投影，照推来的 `turn.started`、`turn.ended` 记。
    running: HashSet<String>,
}

impl Glances {
    /// 没定下一次时隔 `default`。
    pub(super) fn new(default: Duration) -> Glances {
        Glances {
            default,
            ..Glances::default()
        }
    }
}

impl Route {
    /// 卡片变了（存卡片的几处叫它）：下一次 [`Route::sync_glances`] 重新对账。
    pub(super) fn glances_changed(&mut self) {
        self.glances.synced = false;
    }

    /// 没对过账的对一次账（主循环每办完一件、起来订阅完以后叫它，同场所开着的插件）。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn sync_glances(&mut self) -> Result<(), Gone> {
        if self.glances.synced {
            return Ok(());
        }
        self.glances.synced = true;
        self.reconcile_glances().await
    }

    /// 照场所规则和自己建的定时对账（模块说明）。列不出定时的记一行、什么都不改。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn reconcile_glances(&mut self) -> Result<(), Gone> {
        let mut known: Vec<String> = self.venues.keys().cloned().collect();
        for venue in self.glances.privates.keys() {
            if !known.contains(venue) {
                known.push(venue.clone());
            }
        }
        let want: Vec<String> = known
            .iter()
            .filter(|venue| self.glance_on(venue))
            .cloned()
            .collect();
        // 一个都不该看、也没记着等着的：不问核心。先前留下的定时到点照开关查，什么都不做就没了。
        if want.is_empty() && self.glances.armed.is_empty() {
            return Ok(());
        }
        let reply = self.core.call("schedule.list", json!({})).await?;
        if let Some(reason) = reason(&reply) {
            tracing::warn!(target: TARGET, reason, "glances not listed");
            return Ok(());
        }
        let have: Vec<(String, Option<Prompt>)> = reply["result"]["schedules"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|one| one["target"]["package"].is_object())
            .filter_map(|one| {
                let id = one["id"].as_str()?.to_string();
                Some((id, one["prompt"].as_str().and_then(Prompt::read)))
            })
            .collect();
        let done = reconcile((&want, &known), &have);
        self.glances.armed = have
            .into_iter()
            .filter(|(id, _)| !done.cancel.contains(id))
            .filter_map(|(id, prompt)| Some((prompt?.venue, id)))
            .collect();
        for id in &done.cancel {
            self.core.call("schedule.cancel", json!({"id": id})).await?;
        }
        let (cancelled, added) = (done.cancel.len(), done.add.len());
        for venue in done.add {
            let after = self.glance_last(&venue);
            let prompt = Prompt {
                venue,
                after,
                seconds: self.glances.default.as_secs(),
            };
            self.arm_glance(prompt, None).await?;
        }
        tracing::info!(target: TARGET, armed = self.glances.armed.len(), cancelled, added, "glances reconciled");
        Ok(())
    }

    /// 场所 `venue` 头一回找到会话 `session`（第 7 条）：私聊的记下会话，该看却还没有定时的补一个。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn glance_found(&mut self, venue: &str, session: &str) -> Result<(), Gone> {
        self.glance_private(venue, session);
        if self.glances.armed.contains_key(venue) || !self.glance_on(venue) {
            return Ok(());
        }
        let prompt = Prompt {
            venue: venue.to_string(),
            after: self.glance_last(venue),
            seconds: self.glances.default.as_secs(),
        };
        self.arm_glance(prompt, None).await
    }

    /// 私聊场所 `venue` 的会话是 `session`（起来时列出来的、头一回找到的）：记下，到点照它开轮。群的不记（群照投影）。
    pub(super) fn glance_private(&mut self, venue: &str, session: &str) {
        if parse(venue).is_some_and(|venue| venue.kind() == VenueKind::Private) {
            self.glances
                .privates
                .insert(venue.to_string(), session.to_string());
        }
    }

    /// 私聊会话 `session` 推来的一条事件 `event`：开轮了记下在跑（结束在 [`Route::glance_turn_ended`]）。
    pub(super) fn glance_heard(&mut self, session: &str, event: &Value) {
        if event["kind"] == "turn.started" {
            self.glances.running.insert(session.to_string());
        }
    }

    /// 她用工具定了场所 `venue` 隔 `minutes` 分钟再看（I-89 下，`qq_glance`）：删掉等着的定时，照它建新的；这一轮是看一眼开的，
    /// 结束时不再照出厂的间隔排。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn glance_after(&mut self, venue: &str, minutes: u64) -> Result<(), Gone> {
        if let Some(id) = self.glances.armed.remove(venue) {
            self.core.call("schedule.cancel", json!({"id": id})).await?;
        }
        let prompt = Prompt {
            venue: venue.to_string(),
            after: self.glance_last(venue),
            seconds: minutes.max(1).saturating_mul(60),
        };
        self.arm_glance(prompt, None).await
    }

    /// 核心推来 `schedule.fired`（`params`）：读出是哪个场所，过一道。读不出的（手造的、别的写法）记一行。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn glance_fired(&mut self, params: &Value) -> Result<(), Gone> {
        let id = params["id"].as_str().unwrap_or_default().to_string();
        let Some(prompt) = params["prompt"].as_str().and_then(Prompt::read) else {
            tracing::warn!(target: TARGET, schedule = %id, "glance prompt unreadable");
            return Ok(());
        };
        if self.glances.armed.get(&prompt.venue) == Some(&id) {
            self.glances.armed.remove(&prompt.venue);
        }
        if let Some(missed) = params["missed"].as_u64() {
            tracing::info!(target: TARGET, venue = %prompt.venue, missed, "glances missed");
        }
        self.glance_due(&id, prompt).await
    }

    /// 会话 `session`（群的主线、私聊）一轮结束了：等着的那一次再过一遍；看一眼开的那一轮没定下一次的照出厂的间隔再排。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn glance_turn_ended(&mut self, session: &str) -> Result<(), Gone> {
        self.glances.running.remove(session);
        if let Some((venue, after)) = self.glances.looking.remove(session)
            && !self.glances.armed.contains_key(&venue)
        {
            let prompt = Prompt {
                venue,
                after,
                seconds: self.glances.default.as_secs(),
            };
            self.arm_glance(prompt, None).await?;
        }
        if let Some((id, prompt)) = self.glances.postponed.remove(session) {
            self.glance_due(&id, prompt).await?;
        }
        Ok(())
    }

    /// 到点的定时 `id`（交回的 `prompt`）过一道（模块说明）。
    async fn glance_due(&mut self, id: &str, prompt: Prompt) -> Result<(), Gone> {
        let Some(venue) = parse(&prompt.venue) else {
            tracing::warn!(target: TARGET, venue = %prompt.venue, "glance venue unreadable");
            return Ok(());
        };
        if !self.glance_on(&prompt.venue) {
            tracing::info!(target: TARGET, venue = %prompt.venue, "glance off");
            return Ok(());
        }
        let applied = self.applied(&venue);
        if let Some(wake) = applied::wakes(&applied) {
            tracing::info!(target: TARGET, venue = %prompt.venue, wake = %wake, "glance after sleep");
            return self.arm_glance(prompt, Some(wake)).await;
        }
        if venue.kind() == VenueKind::Private {
            return self.glance_private_due(id, prompt).await;
        }
        let found = self.venues.get(&prompt.venue).cloned();
        let Some((session, group)) =
            found.and_then(|session| Some((session.clone(), self.groups.get(&session)?)))
        else {
            return self.arm_glance(prompt, None).await;
        };
        if group.busy() {
            tracing::info!(target: TARGET, venue = %prompt.venue, "glance after turn");
            self.glances
                .postponed
                .insert(session, (id.to_string(), prompt));
            return Ok(());
        }
        let to = unanswered(group.heard(), prompt.after);
        let last = group.last();
        let later = Prompt {
            after: last,
            ..prompt.clone()
        };
        if to.is_empty() {
            tracing::debug!(target: TARGET, venue = %prompt.venue, "glance quiet");
            return self.arm_glance(later, None).await;
        }
        let facts = self.glance_fact();
        let tag = Tag {
            id: format!("glance/{id}"),
            number: 0,
            venue: prompt.venue.clone(),
            who: String::new(),
            text: String::new(),
            facts: Vec::new(),
        };
        let count = to.len();
        match self.respond(&session, &to, &facts, &tag).await? {
            Responded::Asked => {
                tracing::info!(target: TARGET, venue = %prompt.venue, count, "glance opened");
                self.glances.looking.insert(session, (prompt.venue, last));
                Ok(())
            }
            Responded::Refused | Responded::Sealed => self.arm_glance(later, None).await,
        }
    }

    /// 私聊到点（I-89 下）：不看有没有新消息，闲着就开一轮、只带那块事实，正在跑的等它结束；找不到会话的照间隔再排。
    async fn glance_private_due(&mut self, id: &str, prompt: Prompt) -> Result<(), Gone> {
        let found = self.glances.privates.get(&prompt.venue);
        let Some(session) = found.or_else(|| self.venues.get(&prompt.venue)).cloned() else {
            return self.arm_glance(prompt, None).await;
        };
        if self.glances.running.contains(&session) {
            tracing::info!(target: TARGET, venue = %prompt.venue, "glance after turn");
            self.glances
                .postponed
                .insert(session, (id.to_string(), prompt));
            return Ok(());
        }
        let facts = self.glance_fact();
        let tag = Tag {
            id: format!("glance/{id}"),
            number: 0,
            venue: prompt.venue.clone(),
            who: String::new(),
            text: String::new(),
            facts: Vec::new(),
        };
        match self.respond(&session, &[], &facts, &tag).await? {
            Responded::Asked => {
                tracing::info!(target: TARGET, venue = %prompt.venue, count = 0, "glance opened");
                self.glances.looking.insert(session, (prompt.venue, 0));
                Ok(())
            }
            Responded::Refused | Responded::Sealed => self.arm_glance(prompt, None).await,
        }
    }

    /// 给 `prompt` 的场所建一个定时：`at` 那一刻，没给的照它的间隔从此刻往后数。建不成的记一行。
    async fn arm_glance(&mut self, prompt: Prompt, at: Option<Timestamp>) -> Result<(), Gone> {
        let Some(at) = at.or_else(|| later(prompt.seconds)) else {
            tracing::warn!(target: TARGET, venue = %prompt.venue, "glance not armed: clock unreadable");
            return Ok(());
        };
        let params = json!({
            "when": {"at": at.to_string()},
            "prompt": prompt.text(),
            "target": {"package": {}},
        });
        let reply = self.core.call("schedule.add", params).await?;
        match reply["result"]["schedule"]["id"].as_str() {
            Some(id) => {
                tracing::info!(target: TARGET, venue = %prompt.venue, schedule = id, at = %at, "glance armed");
                self.glances.armed.insert(prompt.venue, id.to_string());
            }
            None => {
                let reason = reason(&reply).unwrap_or("bad_reply");
                tracing::warn!(target: TARGET, venue = %prompt.venue, reason, "glance not armed");
            }
        }
        Ok(())
    }

    /// 场所 `venue` 该不该看：名单里的群（有它的群卡）、有私聊卡的私聊，规则的 `glance` 开着。
    pub(super) fn glance_on(&mut self, venue: &str) -> bool {
        let Some(venue) = parse(venue) else {
            return false;
        };
        let listed = match venue.kind() {
            VenueKind::Group => self.rules.cards().lists_group(&venue),
            VenueKind::Private => self.rules.cards().lists_private(&venue),
        };
        listed && applied::glances(&self.applied(&venue))
    }

    /// 场所 `venue` 收到的最后一条（主线序号）：还没有投影的是 0。
    fn glance_last(&self, venue: &str) -> u64 {
        self.venues
            .get(venue)
            .and_then(|session| self.groups.get(session))
            .map_or(0, super::projection::Projection::last)
    }

    /// 看一眼那一块事实：渲染不出（照说不会：起来时查过）的记一行，不带。
    fn glance_fact(&self) -> Vec<Value> {
        let fields: [(&str, &str); 0] = [];
        match self
            .rules
            .glance()
            .render(&fields.iter().copied().collect())
        {
            Ok(text) => vec![json!({"kind": GLANCE, "text": text})],
            Err(error) => {
                tracing::warn!(target: TARGET, error = %error, "glance fact not written");
                Vec::new()
            }
        }
    }
}

/// 场所编号的原文读成场所。
fn parse(venue: &str) -> Option<Venue> {
    VenueId::parse(venue).ok().and_then(|id| Venue::parse(&id))
}

/// 此刻往后 `seconds` 秒；钟读不出、算出界的是空的。
fn later(seconds: u64) -> Option<Timestamp> {
    let now = applied::now()?;
    let span = i64::try_from(seconds).ok()?.checked_mul(SECOND)?;
    Timestamp::from_unix_millis(now.unix_millis().checked_add(span)?)
}
