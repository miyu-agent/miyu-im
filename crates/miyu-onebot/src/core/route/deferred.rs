//! 禁言时判了要回的（施工 O-50，`onebot.md` 第一条「禁言」第 2、3 条，2026-10-17 项目主人定）：她被禁言以后
//! `bridge.json` 的 `mute_window_seconds`（出厂 300）以内来的消息照常判，判了要回的不开轮、排在这里；更晚的进站链照旧只记下
//! （被禁言的人暂时离开了群聊）。解禁以后（解禁的通知、到点）照先后回：同一个人的几条并成一条，最多 `mute_replies`（出厂 3）
//! 条；别人的照常算进限流，满了的不出声地丢掉（QQ 里不发提示）。
//!
//! 只在桥的内存里：桥重启就丢（禁言当中重启少见，丢了的只是解禁以后少回几句）。

use miyu_chat::{Ctx, Moderation, Standing, Venue, rate_full};
use miyu_client::ids::ExternalId;

use super::decide::Decision;
use super::judges::Tag;
use super::{Route, applied};
use crate::TARGET;
use crate::core::Gone;

/// 排着的一条：判了要回的那一次。
#[derive(Debug, Clone)]
pub(super) struct Deferred {
    /// 发的人。
    pub(super) sender: ExternalId,
    /// 发的人是谁（限流只管别人）。
    pub(super) standing: Standing,
    /// 判断：开轮照它的几条，贴表情照它。
    pub(super) decision: Decision,
    /// 判的最后一条的命令编号这些。
    pub(super) tag: Tag,
}

/// 照先后把同一个人的并成一条（纯逻辑）：每个人排在他头一条的位置，几条的序号照先后接起来，判断、命令编号取他最后一条的。
pub(super) fn merged(deferred: Vec<Deferred>) -> Vec<Deferred> {
    let mut merged: Vec<Deferred> = Vec::new();
    for one in deferred {
        match merged.iter_mut().find(|kept| kept.sender == one.sender) {
            Some(kept) => {
                let mut msgs = std::mem::take(&mut kept.decision.msgs);
                for seq in &one.decision.msgs {
                    if !msgs.contains(seq) {
                        msgs.push(*seq);
                    }
                }
                kept.decision = one.decision;
                kept.decision.msgs = msgs;
                kept.tag = one.tag;
                kept.standing = one.standing;
            }
            None => merged.push(one),
        }
    }
    merged
}

impl Route {
    /// 群会话 `session` 此刻是不是被禁言着（照投影）：判了要回的这时排着，不开轮。
    pub(super) fn deferring(&self, session: &str) -> bool {
        let Some(clock) = applied::clock() else {
            return false;
        };
        self.groups
            .get(session)
            .is_some_and(|group| group.muted(clock.now).is_some())
    }

    /// 排一条（「禁言」第 2 条）。
    pub(super) fn defer(&mut self, session: &str, deferred: Deferred) {
        tracing::info!(target: TARGET, venue = %deferred.tag.venue, message = deferred.tag.number, "reply deferred while muted");
        self.deferred
            .entry(session.to_string())
            .or_default()
            .push(deferred);
    }

    /// 解禁了：群会话 `session` 排着的照先后回（「禁言」第 3 条）。同一个人的并成一条；最多回 `mute_replies` 条，多的丢掉；别人的
    /// 先看限流（这一刻的投影，前一条开的轮算进去），满了的丢掉（终端管理员、白名单成员本来不受限流，照回）。丢掉的只记运行日志。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn release(&mut self, session: &str) -> Result<(), Gone> {
        let Some(deferred) = self.deferred.remove(session) else {
            return Ok(());
        };
        let mut answered = 0;
        for one in merged(deferred) {
            let venue = one.tag.venue.clone();
            if answered >= self.mute_replies {
                tracing::info!(target: TARGET, venue, message = one.tag.number, why = "too_many", "deferred reply dropped");
                continue;
            }
            self.catch_up(session).await?;
            if one.standing == Standing::Member && self.rate_filled(session) {
                tracing::info!(target: TARGET, venue, message = one.tag.number, why = "rate_limited", "deferred reply dropped");
                continue;
            }
            if self
                .dispatched(session, &one.decision.msgs, &one.tag)
                .await?
            {
                self.react(session, &one.decision);
            }
            answered += 1;
        }
        Ok(())
    }

    /// 群会话 `session` 此刻限流满了没有：照套到这个群上的 `rate`、投影里别人开的回合（同进站链）。不知道是哪个群的当没满。
    fn rate_filled(&mut self, session: &str) -> bool {
        let Some(venue) = self
            .peers
            .get(session)
            .and_then(|peer| Venue::parse(&peer.venue))
        else {
            return false;
        };
        let applied = self.applied(&venue);
        let (Some(clock), Some(group)) = (applied::clock(), self.groups.get(session)) else {
            return false;
        };
        let ctx = Ctx {
            rate: applied::rate(&applied),
            sleep: None,
            allow: None,
            muted: false,
            turns: group.turns(&self.whitelist),
            moderation: Moderation {
                keywords: Vec::new(),
                base64: applied.params.base64,
            },
        };
        rate_full(&ctx, clock)
    }
}
