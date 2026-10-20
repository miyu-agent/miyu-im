//! 她被禁言、解禁（施工 O-25 中，`onebot.md` 第一条「出站队列」第 7 条；施工 O-50 起「禁言」）：NapCat 推来的 `group_ban`（禁的
//! 是她的）记进这个群的会话：禁言记 `ext.im.venues.muted {until, by?}`（`until` 是本机此刻加禁言的秒数，「施工时定的」第 122
//! 条；`by` 是谁禁的号，O-50），解禁记 `ext.im.venues.unmuted {why, by?}`（`why` 是 `lifted`：解禁的通知；`expired`：到点，
//! QQ 不推通知，桥照投影里禁到的时刻醒了补记，O-50）。记下以后把留着的推送收进投影（进站链的 `Ctx.muted`、出站的门都照投影），
//! 再看一遍排着的：解禁了的照先后发，禁言改短了的照新的到期时刻醒。会话照群消息一样找（`session`）；命令编号自己编：平台不重发
//! 通知（同撤回）。
//!
//! O-50（2026-10-17 项目主人定）：禁言那一刻、解禁那一刻各经核心的 `session.note` 给这个群会话记一块事实（`muted`、`unmuted`，
//! 模板是出厂数据，`rules/facts.rs`），她知道自己被谁禁了多久、什么时候能说话了；解禁以后回禁言时排着的（`deferred`）。

use std::collections::BTreeMap;
use std::time::Instant;

use miyu_chat::VenueKind;
use miyu_client::time::Timestamp;
use serde_json::{Value, json};

use super::Route;
use super::applied;
use super::fields::{NAME, clean};
use super::projection::{MUTED, UNMUTED};
use super::queue::until;
use super::session::Place;
use crate::TARGET;
use crate::chat::To;
use crate::core::{Gone, reason};
use crate::toolset::duration;

/// 禁言的事实的 `kind`。
const MUTED_FACT: &str = "muted";

/// 解禁的事实的 `kind`。
const UNMUTED_FACT: &str = "unmuted";

impl Route {
    /// 机器人号 `bot` 在群号是 `group` 的群里被 `operator` 禁言 `seconds` 秒；空的是解禁了。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn muted(
        &mut self,
        (bot, group): (i64, i64),
        seconds: Option<u64>,
        operator: Option<i64>,
    ) -> Result<(), Gone> {
        let venue = match self.platform.venue(VenueKind::Group, group) {
            Ok(venue) => venue,
            Err(error) => {
                tracing::warn!(target: TARGET, group, error = %error, "venue not made, mute not noted");
                return Ok(());
            }
        };
        let place = Place::group(&venue, &self.applied(&venue), bot, group);
        let Some(seconds) = seconds else {
            // 解禁：投影里本来就没禁着的（到点已经补记过、通知重了）不再记（O-50）。
            if !self.found(&place).await? {
                return Ok(());
            }
            let Some(session) = self.venues.get(venue.id().as_str()).cloned() else {
                return Ok(());
            };
            self.catch_up(&session).await?;
            let pending = self
                .groups
                .get(&session)
                .is_some_and(|projection| projection.mute_end().is_some());
            if !pending {
                tracing::info!(target: TARGET, venue = %venue.id(), "unmute ignored, not muted");
                return Ok(());
            }
            return self.unmuted((&session, group), "lifted", operator).await;
        };
        let Some(end) = applied::clock().and_then(|clock| until(clock.now, seconds)) else {
            tracing::warn!(target: TARGET, venue = %venue.id(), seconds, "mute not noted");
            return Ok(());
        };
        let mut body = json!({"until": end});
        if let Some(operator) = operator {
            body["by"] = json!(operator);
        }
        let params = json!({"kind": MUTED, "body": body});
        let Some((session, reply)) = self
            .on_session(&place, None, "events.append", params)
            .await?
        else {
            return Ok(());
        };
        if let Some(reason) = reason(&reply) {
            tracing::warn!(target: TARGET, venue = %venue.id(), reason, "mute refused");
            return Ok(());
        }
        tracing::info!(target: TARGET, venue = %venue.id(), seconds, "mute noted");
        self.catch_up(&session).await?;
        let by = self.operator_name(group, operator);
        let lasting = duration(seconds);
        let fields = BTreeMap::from([("by", by.as_str()), ("duration", lasting.as_str())]);
        let text = self.rules.muted_fact().render(&fields);
        self.note_fact(&session, MUTED_FACT, text).await?;
        self.pump().await
    }

    /// 记解禁：`ext.im.venues.unmuted {why, by?}`，收进投影，记一块解禁的事实，回禁言时排着的，再看一遍出站排着的。`session`
    /// 是群号 `group` 的群会话，`why` 是 `lifted`、`expired`，`operator` 是谁解的。
    async fn unmuted(
        &mut self,
        (session, group): (&str, i64),
        why: &str,
        operator: Option<i64>,
    ) -> Result<(), Gone> {
        let mut body = json!({"why": why});
        if let Some(operator) = operator {
            body["by"] = json!(operator);
        }
        let Some(_) = self.append(session, None, UNMUTED, body).await? else {
            return Ok(());
        };
        tracing::info!(target: TARGET, session, why, "unmute noted");
        self.catch_up(session).await?;
        let by = self.operator_name(group, operator);
        let fields = BTreeMap::from([("why", why), ("by", by.as_str())]);
        let text = self.rules.unmuted_fact().render(&fields);
        self.note_fact(session, UNMUTED_FACT, text).await?;
        self.release(session).await?;
        self.pump().await
    }

    /// 到点的禁言补记解禁（O-50，「禁言」第 1 条：QQ 到点不推通知）：投影里禁到的时刻不晚于此刻、还没记解禁的群会话，一个个记。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn expire_mutes(&mut self) -> Result<(), Gone> {
        let Some(clock) = applied::clock() else {
            return Ok(());
        };
        let ended: Vec<(String, i64)> = self
            .groups
            .iter()
            .filter(|(_, projection)| projection.mute_end().is_some_and(|end| end <= clock.now))
            .filter_map(|(session, _)| match self.peers.get(session)?.to {
                To::Group(group) => Some((session.clone(), group)),
                To::Private(_) => None,
            })
            .collect();
        for (session, group) in ended {
            self.unmuted((&session, group), "expired", None).await?;
        }
        Ok(())
    }

    /// 还没记解禁的禁言里最早到点的那一刻（O-50）：跟核心的那一头睡到那一刻补记。过了点的交回此刻。
    pub(super) fn mute_wake(&self) -> Option<Timestamp> {
        self.groups
            .values()
            .filter_map(super::projection::Projection::mute_end)
            .min()
    }

    /// 谁禁的、谁解的写成什么：群成员缓存里的名字（洗过），没有的写号，不知道是谁的空着。
    fn operator_name(&self, group: i64, operator: Option<i64>) -> String {
        let Some(operator) = operator else {
            return String::new();
        };
        self.members
            .name(group, operator, Instant::now())
            .and_then(|name| clean(name, NAME))
            .unwrap_or_else(|| operator.to_string())
    }

    /// 经核心的 `session.note` 给群会话 `session` 记一块 `kind` 的事实，字是模板换出来的 `text`。换不出来的（照说不会：起来时
    /// 查过模板只要那几个字段）、核心拒了的记一行，不再试。
    async fn note_fact(
        &mut self,
        session: &str,
        kind: &str,
        text: Result<String, impl std::fmt::Display>,
    ) -> Result<(), Gone> {
        let text = match text {
            Ok(text) => text,
            Err(error) => {
                tracing::warn!(target: TARGET, session, kind, error = %error, "fact not written");
                return Ok(());
            }
        };
        let facts: Value = json!([{"kind": kind, "text": text}]);
        let reply = self
            .core
            .call("session.note", json!({"session": session, "facts": facts}))
            .await?;
        match reason(&reply) {
            None => tracing::info!(target: TARGET, session, kind, "fact noted"),
            Some(reason) => tracing::warn!(target: TARGET, session, kind, reason, "note refused"),
        }
        Ok(())
    }
}
