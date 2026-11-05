//! 每人的用量记账（设计 08 第二节第 2 条，施工 I-134）：群会话、支线、私聊推来的 `model.called` 照这一轮回的人记进账本
//! （`crate::spent`）。
//!
//! - 群（主线、支线）：回的人照投影这条线在跑的那一轮（[`Projection::payers`](super::Projection::payers)）。
//! - 私聊：没有投影，这里照推来的事件记：人说的话的序号（最近 [`SAID`] 条），这一轮的 `turn.started` 的触发里有人说的话的，
//!   是人叫的，记到对面那个人头上；看一眼这类桥开的轮次（触发是桥的事实）不记。

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use miyu_client::events::{Body, By, Event};
use miyu_client::ids::ExternalId;

use serde_json::Value;

use super::Route;
use crate::chat::To;
use crate::spent::{Charge, Spent, personal};

/// 私聊里记着最近几条人说的话的序号：排着队接着开的一轮，触发是排着的最后一条，不会隔太远。
const SAID: usize = 64;

/// 记账的一头。
pub(super) struct Charging {
    /// 账本。
    spent: Arc<Spent>,
    /// 私聊会话编号 → 这个私聊的样子。
    private: HashMap<String, Asked>,
}

/// 一个私聊：最近人说的话、这一轮是不是人叫的。
#[derive(Debug, Default)]
struct Asked {
    /// 人说的话的序号，照先后，最多 [`SAID`] 条。
    said: VecDeque<u64>,
    /// 这一轮：回合编号、是不是人叫的。
    turn: Option<(u64, bool)>,
}

impl Asked {
    /// 收一条私聊的事件：是这一轮里人叫的那一轮的主请求，交回真。
    fn take(&mut self, event: &Event) -> bool {
        let turn = event.turn.map(|turn| turn.started().get());
        match &event.body {
            // 主人在私聊里说的，核心认成 `Person`（照主人对应表）；别人的是 `External`。
            Body::MessageUser(_) if matches!(event.by, By::External(_) | By::Person(_)) => {
                self.said.push_back(event.seq.get());
                if self.said.len() > SAID {
                    self.said.pop_front();
                }
                false
            }
            Body::TurnStarted(started) => {
                let asked = started
                    .triggers
                    .iter()
                    .chain(started.trigger.as_ref())
                    .any(|seq| self.said.contains(&seq.get()));
                self.turn = turn.map(|turn| (turn, asked));
                false
            }
            Body::ModelCalled(_) => turn.is_some() && self.turn == turn.map(|turn| (turn, true)),
            _ => false,
        }
    }
}

impl Charging {
    /// 记进 `spent`。
    pub(super) fn new(spent: Arc<Spent>) -> Charging {
        Charging {
            spent,
            private: HashMap::new(),
        }
    }

    /// 群的线 `line`（主线、支线）推来的一条 `event`：报了用量的主请求记给 `payers`（这条线在跑的那一轮回的人）。场所是 `venue`。
    pub(super) fn group(&self, line: &str, venue: &str, event: &Event, payers: &[ExternalId]) {
        if let Some(tokens) = personal(event) {
            self.charge(line, venue, event, tokens, payers);
        }
    }

    /// 私聊会话 `session` 推来的一条 `event`：人叫的那一轮报了用量的主请求记给对面的人 `peer`（认不出的不记）。场所是 `venue`。
    pub(super) fn private(
        &mut self,
        session: &str,
        venue: &str,
        event: &Event,
        peer: Option<ExternalId>,
    ) {
        let asked = self
            .private
            .entry(session.to_string())
            .or_default()
            .take(event);
        if let (true, Some(tokens), Some(peer)) = (asked, personal(event), peer) {
            self.charge(session, venue, event, tokens, &[peer]);
        }
    }

    /// 记一笔。
    fn charge(&self, line: &str, venue: &str, event: &Event, tokens: u64, payers: &[ExternalId]) {
        let charge = Charge {
            session: line,
            seq: event.seq.get(),
            venue,
            at: event.at.unix_millis(),
            tokens,
        };
        self.spent.charge(&charge, payers, now());
    }
}

impl Route {
    /// 私聊会话 `session` 推来的一条 `raw`（施工 I-134）：只读这里用得上的三种，别的不解析。
    pub(super) fn charge_privately(&mut self, session: &str, raw: &Value) {
        if !matches!(
            raw["kind"].as_str(),
            Some("message.user" | "turn.started" | "model.called")
        ) || raw["seq"].as_u64().is_none()
        {
            return;
        }
        let Ok(event) = serde_json::from_value::<Event>(raw.clone()) else {
            return;
        };
        let Some(peer) = self.peers.get(session) else {
            return;
        };
        let person = match &peer.to {
            To::Private(user) => peer.platform.person(user).ok(),
            To::Group(_) => None,
        };
        let venue = peer.venue.as_str().to_string();
        self.limits
            .charging
            .private(session, &venue, &event, person);
    }
}

/// 此刻，毫秒。
pub(super) fn now() -> i64 {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests;
