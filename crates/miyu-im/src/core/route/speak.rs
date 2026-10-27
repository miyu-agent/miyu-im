//! 她的话发回去（施工 O-23，`onebot.md` 第一条「群里怎么叫她」第 1、9 条；O-25 上接上出站链，私聊的「怎么走」第 10 条也在
//! 这里）：群会话推来的事件（订阅时从头补来的、之后推来的）走同一条路收进这个群的投影（`projection`）；补来的她的话交给补来的
//! 那一段（`backlog`，施工 O-32：期限以内、没入队的由 `revive` 补发），补完以前推来的新的也是（施工 O-32 补）；之后她新说的话
//! 先过出站链（`outbound`），丢了的记一行，过了的照纯文本拆段，一段一条入队（`sending`，O-25 中：先记
//! `ext.im.venues.queued` 再照先后交 NapCat，群里的成了记 `venue.delivered`），第一段带引用和 @。私聊里她的话也过出站链、
//! 拆段、入队，不记 `venue.delivered`。这一轮她调过 `skip_reply` 的（施工 O-26，`quiet`）不过链、不入队，记一行
//! `reply dropped why=skipped`。支线推来的（施工 O-36）由 `lanes.rs` 收进主线的投影，她在支线说的话也从这里照主线发。

use miyu_chat::{OutCtx, Venue};
use miyu_client::events::Event;
use miyu_client::ids::VenueId;
use serde_json::Value;

use super::backlog::queued_reply;
use super::outbound::{Passed, group_ctx, pass, private_ctx, why_name};
use super::projection::{Aim, Speaking};
use super::quiet::{Held, has_calls};
use super::sending::{Body, Piece, What};
use super::voicing::Voice;
use super::{Route, applied, reply_text};
use crate::TARGET;
use crate::core::Gone;

impl Route {
    /// 群会话 `session` 推来的一条 `pushed`（`event` 推送）：落了盘的收进投影（瞬时的没有序号，不看），交给贴着的表情看要不要摘
    /// （施工 O-25 下，`reaction`）、交给 `quiet` 看这一轮说不说话（施工 O-26），她新说的话发回群里，这一轮不说话了的不发。
    ///
    /// # Errors
    ///
    /// 她的话入队时写不出去、等的时候核心断开（施工 O-25 中）。
    pub(super) async fn heard(&mut self, session: &str, pushed: &Value) -> Result<(), Gone> {
        let raw = &pushed["params"]["event"];
        if raw["seq"].as_u64().is_none() {
            return Ok(());
        }
        let event: Event = match serde_json::from_value(raw.clone()) {
            Ok(event) => event,
            Err(error) => {
                tracing::warn!(target: TARGET, session, error = %error, "event not understood");
                return Ok(());
            }
        };
        let Some(group) = self.groups.get_mut(session) else {
            return Ok(());
        };
        let speaking = group.take(&event);
        self.reactions.seen(session, &event);
        let quiet = self.quiet.heard(session, raw);
        let backlog = self
            .backlogs
            .get_mut(session)
            .filter(|backlog| backlog.replayed(raw));
        let replayed = backlog.is_some();
        if let Some(backlog) = backlog {
            backlog.take(raw);
        }
        // 她给人看的文件、发出去的图发进群里（施工 O-46，`shown`；O-51，`stickers`）：补来的、这一轮不说话的不发。
        if !replayed && !quiet {
            self.show((session, session), raw);
            self.stickers((session, session), raw).await?;
        }
        match speaking {
            // 补来的不照常发：这一轮没说不说话的交给补来的那一段，期限以内、没入队的补完了补发（施工 O-32，`revive`）。补完以前
            // 推来的新的也一样（施工 O-32 补）：核心带 `after` 订阅时回应以前就可能推来，起来时列出来的会话这时还不知道发到哪，
            // 照常发就丢了；交给补来的那一段，补完了认出发到哪再照先后发。
            Some(speaking)
                if speaking.replayed || (self.backlogs.contains_key(session) && !quiet) =>
            {
                // 仅发送最终消息的场所（施工 O-57）：中间那几步不补。
                let middle = has_calls(raw) && self.final_only(session);
                if let Some(backlog) = self.backlogs.get_mut(session)
                    && !quiet
                    && !middle
                {
                    backlog.said(raw, reply_text(raw), Some(speaking));
                }
                Ok(())
            }
            Some(_) if quiet => {
                self.hushed(session, &reply_text(raw));
                Ok(())
            }
            Some(speaking) => {
                let voice = Voice::Group(speaking);
                self.say_or_hold((session, session), raw, voice, reply_text(raw))
                    .await
            }
            None if !replayed => self.turn_closed(session, raw).await,
            None => Ok(()),
        }
    }

    /// 场所会话 `session` 仅发送最终消息（施工 O-57，场所规则的 `outbound.final_only`：群出厂开、私聊出厂关）。
    pub(super) fn final_only(&mut self, session: &str) -> bool {
        let Some(venue) = self
            .peers
            .get(session)
            .and_then(|peer| Venue::parse(&peer.venue))
        else {
            return false;
        };
        self.applied(&venue).params.final_only
    }

    /// 她在线 `line`（主线、支线、私聊）说的一段 `text`，发回会话 `session`（施工 O-57）：仅发送最终消息的场所里，带工具调用的
    /// 回复先扣着（同一回合后面再扣的换掉它，空的不扣），不带工具调用的当场发、扣着的丢掉（它没有字的发扣着的）；别的场所当场发。
    ///
    /// # Errors
    ///
    /// 同 [`Route::voice`]。
    pub(super) async fn say_or_hold(
        &mut self,
        (session, line): (&str, &str),
        raw: &Value,
        voice: Voice,
        text: String,
    ) -> Result<(), Gone> {
        if let Some(turn) = raw["turn"].as_u64()
            && self.final_only(session)
        {
            if has_calls(raw) {
                if !text.trim().is_empty() {
                    let held = Held {
                        session: session.to_string(),
                        voice,
                        text,
                    };
                    self.quiet.hold(line, turn, held);
                }
                return Ok(());
            }
            // 不带工具调用的是最后那段：扣着的丢掉；这一段没有字的（字都在前一步里），照扣着的发。
            if let Some(held) = self.quiet.release(line, turn)
                && text.trim().is_empty()
            {
                return self.voice(&held.session, held.voice, held.text).await;
            }
        }
        self.voice(session, voice, text).await
    }

    /// 线 `line` 推来的一条是回合结束（`turn.ended`）的：仅发送最终消息时扣着的那段（她最后一步带着工具调用，例如发表情包结束
    /// 这一轮）这时发出去（施工 O-57）。
    ///
    /// # Errors
    ///
    /// 同 [`Route::voice`]。
    pub(super) async fn turn_closed(&mut self, line: &str, raw: &Value) -> Result<(), Gone> {
        if raw["kind"] != "turn.ended" {
            return Ok(());
        }
        if let Some(turn) = raw["turn"].as_u64()
            && let Some(held) = self.quiet.release(line, turn)
        {
            self.voice(&held.session, held.voice, held.text).await?;
        }
        // 群的主线这一轮结束了：排着的开下一轮（施工 I-16，`waiting`）。
        if self.groups.contains_key(line) {
            self.next_waiting(line).await?;
        }
        Ok(())
    }

    /// 会话 `session` 这一轮不说话了，她说的 `text` 不发：记一行 `reply dropped`（`why=skipped`，不记原文，施工 O-26）。
    pub(super) fn hushed(&self, session: &str, text: &str) {
        if let Some(peer) = self.peers.get(session) {
            let chars = text.chars().count();
            tracing::info!(target: TARGET, venue = %peer.venue, why = "skipped", chars, "reply dropped");
        }
    }

    /// 把群会话 `session` 留着没办的推送照先后收进投影（第 3 条：判以前；第 1 条：订阅以后），它的几条支线推来的也一起（施工
    /// O-36，`lanes.rs`）。
    ///
    /// # Errors
    ///
    /// 同 [`Route::heard`]。
    pub(super) async fn catch_up(&mut self, session: &str) -> Result<(), Gone> {
        let lanes = self
            .groups
            .get(session)
            .map(|group| group.lanes())
            .unwrap_or_default();
        let wanted = |one: &str| one == session || lanes.iter().any(|lane| lane == one);
        for pushed in self.core.take_events_of(wanted) {
            match pushed["params"]["session"].as_str() {
                Some(one) if one == session => self.heard(session, &pushed).await?,
                Some(lane) => {
                    let lane = lane.to_string();
                    self.lane_heard(session, &lane, &pushed).await?;
                }
                None => {}
            }
        }
        Ok(())
    }

    /// 群里她的一段话 `text` 过出站链（情形照投影交出的 `speaking`、本机此刻、这个群此刻的参数），过了的一段一条入队。本机的钟读
    /// 不出的（照说不会）记一行、不发。
    pub(super) async fn speak_now(
        &mut self,
        session: &str,
        speaking: Speaking,
        text: &str,
    ) -> Result<(), Gone> {
        let Some(peer) = self.peers.get(session).cloned() else {
            unknown_peer(session, text);
            return Ok(());
        };
        let Some(venue) = Venue::parse(&peer.venue) else {
            return Ok(());
        };
        let Some(clock) = applied::clock() else {
            tracing::warn!(target: TARGET, venue = %peer.venue, "clock not readable, reply dropped");
            return Ok(());
        };
        let params = self.applied(&venue).params;
        let ctx = group_ctx(&speaking, clock.now, params.outbound);
        let aim = speaking.aim.as_ref();
        let Some(passed) = passed(&peer.venue, text, &ctx, aim, params.split_chars) else {
            return Ok(());
        };
        let what = What::Reply {
            line: speaking.line,
            turn: speaking.turn,
            to: speaking.to,
        };
        self.say(session, passed, what).await
    }

    /// 私聊会话 `session` 推来的一条事件 `event`：先交给 `quiet` 看这一轮说不说话（施工 O-26），人说的话记下引用（施工 O-31）；是她的回话的（「怎么走」第 10
    /// 条），这一轮不说话了的不发，别的过出站链（两样都是假，这一轮发出去的照桥入队时自己记的 `Spoken`），过了的一段一条入队。
    ///
    /// # Errors
    ///
    /// 同 [`Route::heard`]。
    pub(super) async fn say_privately(&mut self, session: &str, event: &Value) -> Result<(), Gone> {
        let quiet = self.quiet.heard(session, event);
        // 驱动的工具照回合数一轮调了几次（施工 O-35、T-3）：私聊没有投影，推来的事件的回合记下来。
        if let Some(turn) = event["turn"].as_u64() {
            self.turns.insert(session.to_string(), Some(turn));
        }
        if event["kind"] == "message.user" {
            // 平台工具（一）照它找私聊里叫她做的那条引用的是哪一条（施工 O-31，`acting`）。
            let quote = event["body"]["venue"]["reply_to"]
                .as_str()
                .map(str::to_string);
            self.quotes.insert(session.to_string(), quote);
        }
        // 仅发送最终消息的私聊（施工 O-57）：补来的中间那几步不补。
        let middle = has_calls(event) && self.final_only(session);
        if let Some(backlog) = self.backlogs.get_mut(session)
            && event["seq"].as_u64().is_some()
        {
            // 补来的（桥起来时从头订阅，施工 O-32）：不照常发。这一轮入队了的照日志补回 `Spoken`，重启以后同一轮里照旧去重；她的话
            // 交给补来的那一段，期限以内、没入队的补完了补发（`revive`）。补完以前推来的新的（施工 O-32 补，同群里的 `heard`）
            // 只把她的话交过去：这时还不知道发到哪。
            if backlog.replayed(event) {
                backlog.take(event);
                if let Some((turn, text)) = queued_reply(event) {
                    self.spoken
                        .entry(session.to_string())
                        .or_default()
                        .add(turn, text);
                }
            }
            if event["kind"] == "message.assistant" && !quiet && !middle {
                backlog.said(event, reply_text(event), None);
            }
            return Ok(());
        }
        // 她给人看的文件、发出去的图发进私聊（施工 O-46，`shown`；O-51，`stickers`）：补来的上面已经交回去了，这一轮不说话的不发。
        if !quiet {
            self.show((session, session), event);
            self.stickers((session, session), event).await?;
        }
        if event["kind"] != "message.assistant" {
            return self.turn_closed(session, event).await;
        }
        if quiet {
            self.hushed(session, &reply_text(event));
            return Ok(());
        }
        // 先问出站挂接点（施工 O-43 下，`voicing`），再 `say_privately_now`；仅发送最终消息的先扣着中间那几步（施工 O-57）。
        let turn = event["turn"].as_u64().unwrap_or(0);
        let voice = Voice::Private { turn };
        self.say_or_hold((session, session), event, voice, reply_text(event))
            .await
    }

    /// 私聊里她第 `turn` 个回合的一段话 `text` 过出站链（去重照这一回合发过的，`Spoken`），过了的一段一条入队。
    ///
    /// # Errors
    ///
    /// 入队时写不出去、核心断开。
    pub(super) async fn say_privately_now(
        &mut self,
        session: &str,
        turn: u64,
        text: &str,
    ) -> Result<(), Gone> {
        let Some(peer) = self.peers.get(session).cloned() else {
            unknown_peer(session, text);
            return Ok(());
        };
        let Some(venue) = Venue::parse(&peer.venue) else {
            return Ok(());
        };
        let params = self.applied(&venue).params;
        let spoken = self.spoken.get(session).map(|spoken| spoken.of(turn));
        let ctx = private_ctx(spoken.unwrap_or_default(), params.outbound);
        let Some(passed) = passed(&peer.venue, text, &ctx, None, params.split_chars) else {
            return Ok(());
        };
        let what = What::Reply {
            line: session.to_string(),
            turn,
            to: Vec::new(),
        };
        self.say(session, passed, what).await
    }

    /// 把过了链的 `passed` 一段一条照先后入队（施工 O-25 中，`sending`），第一段带上引用和 @；她这一轮在这条线上用 `mention`
    /// 指定了要 @ 的（施工 O-55，`aims`），第一段也带上、用掉；用 `quote` 指定了要引用的（施工 I-31），第一段引用那一条。她挑了
    /// 其中哪一样，这一句都不再自动引用、自动 @（施工 I-45）。照场所参数是长文的先试着画成图（施工 O-59 上）。
    pub(super) async fn say(
        &mut self,
        session: &str,
        passed: Passed,
        what: What,
    ) -> Result<(), Gone> {
        let mut lead = passed.lead.clone();
        if let What::Reply { line, turn, .. } = &what {
            let aim = self.take_aim(line, *turn);
            // 她自己挑了引用（施工 I-31）或者 @ 了人（施工 I-45）的：照她挑的，这一句不再自动引用、自动 @（自动引用的那一条
            // 和她 @ 的人会对不上）。
            if aim.quote.is_some() || !aim.users.is_empty() {
                lead.reply = aim.quote;
                lead.at = None;
            }
            lead.more = aim.users;
        }
        // 长文画成图（施工 O-59 上，`long`）：画成了的一页一段入队；不是长文、画不成的照拆段。
        if self.say_long(session, (&passed, &what), &mut lead).await? {
            return Ok(());
        }
        for (index, text) in passed.pieces.into_iter().enumerate() {
            let piece = Piece {
                what: what.clone(),
                text,
                lead: std::mem::take(&mut lead),
                body: Body::Text,
                pause: passed.pauses.get(index).copied().unwrap_or_default(),
            };
            self.enqueue(session, None, piece).await?;
        }
        Ok(())
    }
}

/// 她在会话 `session` 说的一句 `text` 不知道发到哪（例如起来时列出来的会话日志里认不出机器人号、那个场所还没来过消息；补完以前
/// 推来的照补发那条路，不走到这里，施工 O-32 补）：记一行 `reply dropped`（`why=unknown_peer`，不记原文），不悄悄丢。
fn unknown_peer(session: &str, text: &str) {
    let chars = text.chars().count();
    tracing::warn!(target: TARGET, session, why = "unknown_peer", chars, "reply dropped");
}

/// 她在场所 `venue` 的一句 `text` 过出站链（情形 `ctx`，她回的那一条 `aim`，一段最多 `split_chars` 个字符）：丢了的记一行
/// `reply dropped`（原因，不记原文）、交回空的（第 9 条第 2 项）。
fn passed(
    venue: &VenueId,
    text: &str,
    ctx: &OutCtx,
    aim: Option<&Aim>,
    split_chars: usize,
) -> Option<Passed> {
    match pass(text, ctx, aim, split_chars) {
        Ok(passed) => Some(passed),
        Err(why) => {
            let chars = text.chars().count();
            tracing::info!(target: TARGET, venue = %venue, why = why_name(why), chars, "reply dropped");
            None
        }
    }
}
