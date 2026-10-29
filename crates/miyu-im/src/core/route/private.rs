//! 一条私聊（`onebot.md` 第一条「怎么走」第 6 到 8 条，中间是「斜杠命令」；施工 O-36 从 `route.rs` 挪出来，那一份行数到了上限）：
//! 场所、平台上的人经群聊内核拼，拼不出来的记一行、这条不送；带上场所的格，`show_ids` 照这个私聊套出来的场所规则；白名单成员的
//! 私聊照场所规则带人格、预设、工作区（施工 O-27）；陌生人照「其他私聊」卡过进站链（施工 O-60，`strangers`）；只有图、引用的照取图（施工 O-33），合并转发照群消息的写法（施工 O-34）。

use miyu_chat::{Outcome, VenueKind, Why};
use serde_json::json;

use super::noted::recorded;

use super::fields::{Flags, fields};
use super::forwards::{Forwards, with_placeholders};
use super::pictures::Pictures;
use super::session::Place;
use super::strangers::Gating;
use super::{Message, Route, applied};
use crate::TARGET;
use crate::chat::{Platform, Posted};
use crate::core::{Gone, reason};
use crate::recent::{Note, now};

impl Route {
    /// 平台 `platform` 上的一条私聊（第 6 到 8 条，中间是「斜杠命令」）。
    pub(super) async fn private(&mut self, platform: Platform, posted: Posted) -> Result<(), Gone> {
        let made = platform
            .venue(VenueKind::Private, &posted.user)
            .and_then(|venue| Ok((venue, platform.person(&posted.user)?)));
        let (venue, external) = match made {
            Ok(made) => made,
            Err(error) => {
                tracing::warn!(target: TARGET, user = %posted.user, message = posted.message_id, error = %error, "venue not made, message dropped");
                return Ok(());
            }
        };
        // 只有图、引用的私聊照取图（施工 O-33）；带了东西的、没有字也送（施工 O-35：她看得到记号、能取）。合并转发照群消息的写法
        // 在正文后面写占位、交以前展开（施工 O-34），只有转发的也送。
        let pictures = Pictures::of(posted.bot.clone(), posted.message_id, &posted.segments);
        let text = with_placeholders(&posted.text, posted.segments.forwards.len());
        if text.trim().is_empty() && pictures.own.is_empty() && !pictures.any() {
            tracing::debug!(target: TARGET, venue = %venue.id(), message = posted.message_id, "nothing to send");
            return Ok(());
        }
        // 聊天记录（设计 04，施工 C-1 上）：谁的私聊都记，接不接是后面的事。
        self.record_heard((platform, &venue), &external, &posted, &text);
        // 陌生人（施工 O-60，`strangers`）：不在白名单里、「其他私聊」卡让接的才问是不是终端管理员（卡关着的照原来办，不多问一趟），
        // 是的这张卡不再管他；不是的照这张卡过进站链。
        let stranger = !self.whitelisted(&external)
            && applied::allow(&self.applied(&venue)) == Some(true)
            && match self.terminal_admin(external.as_str()).await? {
                Some(true) => {
                    self.rules.add_admin(external.as_str().to_string());
                    false
                }
                Some(false) => true,
                // 问不到的照原来办：核心那头不接陌生人。
                None => false,
            };
        // 私聊每一条都开回合：不写 `ambient`、`asleep`（「施工时定的」第 68 条）；陌生人睡觉时间、限流满了的只记下（施工 O-60）。
        let applied = self.applied(&venue);
        let mut flags = Flags {
            show_ids: applied::show_ids(&applied),
            ..Flags::default()
        };
        let mut taken = self.whitelisted(&external);
        // 只记下的是为什么（最近处理，施工 I-56）。
        let mut only_recorded = None;
        if stranger {
            match self.stranger(&venue, &external, &text) {
                Outcome::Pass => taken = true,
                Outcome::RecordOnly(Why::NotAllowed) => {}
                Outcome::RecordOnly(why) => {
                    taken = true;
                    flags.ambient = true;
                    flags.asleep = why == Why::Asleep;
                    only_recorded = Some(why);
                    tracing::info!(target: TARGET, venue = %venue.id(), why = ?why, "stranger recorded only");
                }
            }
        }
        // 白名单成员卡上关了忽略限流、忽略睡眠的（施工 I-7 中三下一，出厂、没写的都忽略）：只管关了的那一样，满了、睡着的只记下。
        if self.whitelisted(&external) {
            let rate = applied::flag(&applied, "ignore_rate") == Some(false);
            let sleep = applied::flag(&applied, "ignore_sleep") == Some(false);
            if (rate || sleep)
                && let Outcome::RecordOnly(why) =
                    self.gated(&venue, &external, &text, Gating::Listed { rate, sleep })
            {
                flags.ambient = true;
                flags.asleep = why == Why::Asleep;
                only_recorded = Some(why);
                tracing::info!(target: TARGET, venue = %venue.id(), why = ?why, "listed recorded only");
            }
        }
        // 白名单成员、照卡片接的陌生人的私聊会话关在沙盒里、照场所规则造（施工 O-27，同群；O-60）。
        let listed = taken.then_some(&applied);
        let my_name = self.my_name((platform, posted.bot.clone()), None).await;
        let heard = (
            Note::who(posted.name.as_deref(), external.as_str()),
            Note::excerpt(&text),
        );
        let message = Message {
            id: platform.command_id(&posted.bot, posted.message_id, posted.time),
            number: posted.message_id,
            text,
            acting: json!({"external": external}),
            fields: fields(&posted, &[], flags, my_name.as_deref(), None),
            place: Place::private(
                (platform, &venue),
                &external,
                (posted.bot.clone(), posted.user.clone()),
                listed,
            ),
            pictures: Some(pictures),
            forwards: Forwards::of(
                posted.bot.clone(),
                None,
                posted.message_id,
                &posted.segments,
            ),
        };
        // 附加提示词放进 system（施工 O-58 上，`titled`）：先找到会话、放好，这一条开的轮就带上。找过的会话照新的换；没找过的
        // 照规则接的（白名单成员、照卡片接的陌生人）先找：交的时候本来也要找，找到的记着，不多问一次；没有提示词的也放（交 `null`
        // 去掉卡片存了以后忘掉的会话上留着的那一层，O-60）。终端管理员照旧交的时候才找。
        let note = applied::extra_prompt(&applied).map(str::to_string);
        let venue = message.place.peer.venue.to_string();
        let known = self.venues.contains_key(&venue);
        if (known || (message.place.listed() && self.found(&message.place).await?))
            && let Some(session) = self.venues.get(&venue).cloned()
        {
            self.noted(&session, note.as_deref()).await?;
        }
        let submitted = self.submit(message).await?;
        // 会话浮窗的最近处理（施工 I-56，10-27 项目主人：私聊卡片没日志）：私聊不判，核心收下了的记一笔，交给她的写 `private`，
        // 只记下的写为什么。命令、不接的、被拒的不记。
        if let Some((_, reply)) = &submitted
            && reason(reply).is_none()
        {
            let (outcome, why) = match only_recorded {
                None => ("reply", "private"),
                Some(why) => ("record", recorded(why)),
            };
            let (who, text) = heard;
            let note = Note {
                at: now(),
                who,
                text,
                outcome: outcome.to_string(),
                why: json!({"reason": why}),
            };
            self.recent.note(&venue, note);
        }
        Ok(())
    }
}
