//! 撤回（`onebot.md` 第一条「撤回」，施工 O-22）：群里的（`group_recall`）、私聊的（`friend_recall`）都记
//! `events.append {session, kind: "venue.recalled", body: {msg, by}}`：被撤的平台编号、谁撤的平台身份。会话照群消息、私聊
//! 一样找（私聊的陌生人照旧不接，白名单成员的照接，施工 O-27）；会话不在了再找一次。命令编号自己编：平台不重发通知
//! （「施工时定的」第 70 条）。
//!
//! 群里叫她的那一条被撤回了（施工 I-16，照旧版 `message_recall.rs`）：在判的放下、排着的去掉，主线这一轮只在回它的打断
//! （`session.interrupt`，排着队的留着）。

use miyu_chat::VenueKind;
use serde_json::json;

use super::Route;
use super::session::Place;
use crate::TARGET;
use crate::chat::Recall;
use crate::core::{Gone, reason};

impl Route {
    /// 一次撤回 `recall`。
    pub(super) async fn recalled(&mut self, recall: Recall) -> Result<(), Gone> {
        let (kind, number) = match recall.group {
            Some(group) => (VenueKind::Group, group),
            None => (VenueKind::Private, recall.user),
        };
        let made = self.platform.venue(kind, number).and_then(|venue| {
            Ok((
                venue,
                self.platform.person(recall.user)?,
                self.platform.person(recall.by)?,
            ))
        });
        let (venue, user, by) = match made {
            Ok(made) => made,
            Err(error) => {
                tracing::warn!(target: TARGET, user = recall.user, message = recall.message_id, error = %error, "venue not made, recall dropped");
                return Ok(());
            }
        };
        // 聊天记录（设计 04，施工 C-1 上）：撤回不删，那一条标上撤回了、谁撤的，另记一件事。
        self.record_happening(
            (venue.id().as_str(), crate::records::RECALLED),
            (Some(recall.user), Some(recall.by)),
            (None, Some(recall.message_id)),
        );
        let applied = self.applied(&venue);
        let place = match recall.group {
            Some(group) => Place::group(&venue, &applied, recall.bot, group),
            None => {
                let listed = self.whitelisted(&user).then_some(&applied);
                Place::private(&venue, &user, (recall.bot, recall.user), listed)
            }
        };
        let body = json!({"msg": recall.message_id.to_string(), "by": by});
        let params = json!({"kind": "venue.recalled", "body": body});
        let Some((_, reply)) = self
            .on_session(&place, None, "events.append", params)
            .await?
        else {
            return Ok(());
        };
        let message = recall.message_id;
        match reason(&reply) {
            None => tracing::info!(target: TARGET, venue = %venue.id(), message, "recall noted"),
            Some(reason) => {
                tracing::warn!(target: TARGET, venue = %venue.id(), message, reason, "recall refused");
            }
        }
        if recall.group.is_some()
            && let Some(session) = self.venues.get(venue.id().as_str()).cloned()
        {
            self.withdraw(&session, &message.to_string()).await?;
        }
        Ok(())
    }

    /// 群会话 `session` 里平台编号是 `msg` 的那一条被撤回了：不再回它（模块的说明）。
    async fn withdraw(&mut self, session: &str, msg: &str) -> Result<(), Gone> {
        let Some((seq, only)) = self
            .groups
            .get(session)
            .and_then(|group| group.withdrawn(msg))
        else {
            return Ok(());
        };
        let judging = self.judges.cancel(session, seq).is_some();
        let queued = self.queued.drop_msg(session, seq);
        if judging || queued > 0 {
            tracing::info!(target: TARGET, session, message = msg, judging, queued, "reply withdrawn");
        }
        if only {
            let params = json!({"session": session, "queued": "keep"});
            let reply = self.core.call("session.interrupt", params).await?;
            match reason(&reply) {
                None => {
                    tracing::info!(target: TARGET, session, message = msg, "turn interrupted after recall")
                }
                Some(reason) => {
                    tracing::debug!(target: TARGET, session, message = msg, reason, "turn not interrupted")
                }
            }
        }
        Ok(())
    }
}
