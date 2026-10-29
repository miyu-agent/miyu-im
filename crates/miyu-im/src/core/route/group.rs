//! 群消息（`onebot.md` 第一条「群消息」，施工 O-22）：群里的每一条消息都记进这个群的场所会话，一律旁听，交的时候不开回合
//! （18 第五节）；记下了交给 `called` 判要不要叫她（施工 O-23，「群里怎么叫她」）。
//!
//! 1. 场所、平台上的人经群聊内核拼（`onebot::venue`、`onebot::person`），拼不出的记一行、这条不送。
//! 2. 场所规则套到这个群上（`applied`）：找会话带的人格、预设、工作区，发的人是不是管理的人，睡没睡，看不看得到号。
//! 3. 发的人的名字、身份（施工 O-31）顺手记进群成员的缓存；正文里的 @ 写成名字（`names`）。
//! 4. 正文空白、又没有带的东西的不送；只有带的东西的照样交（「施工时定的」第 67 条）。冲她来的（@ 她、引用她、关键词开头）
//!    交以前取这一条和它引用的那一条的图，`attachments` 带上（施工 O-33，`pictures`）。
//! 5. 照私聊的办法交（`Route::submit`）：`/` 开头的先当斜杠命令，回执发回群里；别的 `session.send` 带场所的格。
//! 6. 核心记下了（回应交回序号）的交给 `called` 判（施工 O-23）；挂了进站挂接点的先问它（施工 O-43 中，`steering`）。判以前该问
//!    群名的问、放进 system（施工 O-49，`titled`）。
//!
//! 带合并转发的（施工 O-34）：正文里照旧是占位，交以前展开、写进场所的格（`forwards`）。

use std::time::Instant;

use miyu_chat::{VenueKind, addressed};
use serde_json::{Value, json};

use super::called::Heard;
use super::fields::{Flags, fields};
use super::forwards::Forwards;
use super::pictures::Pictures;
use super::session::Place;
use super::{Message, Route, applied};
use crate::TARGET;
use crate::chat::{Id, Platform, Posted, To};
use crate::core::Gone;

/// 问进站挂接点时带上的东西（施工 O-52，`extending.md` 第二条「`chat.inbound`」）：照先后，第几样（从 1 数，同核心的 `#n`）、
/// 种类、平台的编号（`chat.action` 的 `get_image`、`get_file` 认它）、名字、字节数（有的才写）。别的包（表情包包偷图这类）照它自己去取。
fn inbound_media(posted: &Posted) -> Value {
    let items: Vec<Value> = posted
        .segments
        .media
        .iter()
        .enumerate()
        .map(|(index, media)| {
            let mut item = json!({"index": index + 1, "kind": media.kind.as_str(), "id": media.id});
            if let Some(name) = &media.name {
                item["name"] = json!(name);
            }
            if let Some(size) = media.size {
                item["size"] = json!(size);
            }
            item
        })
        .collect();
    Value::Array(items)
}

impl Route {
    /// 平台 `platform` 上群号是 `group` 的群里的一条消息 `posted`（「群消息」第 3 到 7 条）。
    pub(super) async fn group(
        &mut self,
        platform: Platform,
        group: Id,
        posted: Posted,
    ) -> Result<(), Gone> {
        let made = platform
            .venue(&posted.bot, VenueKind::Group, &group)
            .and_then(|venue| Ok((venue, platform.person(&posted.user)?)));
        let (venue, external) = match made {
            Ok(made) => made,
            Err(error) => {
                tracing::warn!(target: TARGET, group = %group, user = %posted.user, message = posted.message_id, error = %error, "venue not made, message dropped");
                return Ok(());
            }
        };
        let applied = self.applied(&venue);
        if let Some(name) = &posted.name {
            self.members.remember(
                (platform, group.clone(), posted.user.clone()),
                name.clone(),
                Instant::now(),
            );
        }
        if let Some(rank) = posted.rank {
            // 身份顺手记下（施工 O-31，「平台工具（一）」第 7 条；O-48 起禁言不再看它）。
            self.members.ranked(
                (platform, group.clone(), posted.user.clone()),
                rank,
                Instant::now(),
            );
        }
        let text = self.named((platform, group.clone()), &posted).await;
        if text.trim().is_empty() && posted.segments.media.is_empty() {
            tracing::debug!(target: TARGET, venue = %venue.id(), message = posted.message_id, "nothing to send");
            return Ok(());
        }
        let at = posted.segments.at();
        // @ 了的别人；拼不出平台身份的（号是整数，照说不会）不记。
        let mentions: Vec<_> = at
            .iter()
            .filter(|user| **user != posted.bot)
            .filter_map(|user| platform.person(user).ok())
            .collect();
        let flags = Flags {
            mentions_me: at.contains(&posted.bot),
            ambient: true,
            asleep: applied::asleep(&applied),
            show_ids: applied::show_ids(&applied),
        };
        // 冲她来的取这一条和它引用的那一条的图（施工 O-33，`pictures`）：照「群里怎么叫她」第 4 条认（引用她不算叫她，施工 I-20）；
        // 引用她照这个群的投影认，交给挂接点、续聊（引用的不是她的才算对别人说）。
        let quotes_me = posted.segments.reply_to.as_deref().is_some_and(|msg| {
            self.venues
                .get(venue.id().as_str())
                .and_then(|session| self.groups.get(session))
                .is_some_and(|group| group.mine(msg))
        });
        let keywords = applied::keywords(&applied);
        let my_name = self
            .my_name((platform, posted.bot.clone()), Some(group.clone()))
            .await;
        // 群名：场所的格带上给记忆（施工 I-54），交了以后照它放进 system（`titled`）。
        let place = self
            .place(platform, group.clone(), posted.bot.clone())
            .await;
        let calls_her = addressed(VenueKind::Group, flags.mentions_me, &text, &keywords);
        let message = Message {
            id: platform.command_id(&posted.bot, posted.message_id, posted.time),
            number: posted.message_id,
            acting: json!({"external": external, "role": applied::role(&applied, &external)}),
            fields: fields(
                &posted,
                &mentions,
                flags,
                my_name.as_deref(),
                place.as_deref(),
            ),
            place: Place::group(
                (platform, &venue),
                &applied,
                posted.bot.clone(),
                group.clone(),
            ),
            text,
            pictures: calls_her.then(|| {
                let to = To::Group(group.clone());
                Pictures::of(
                    (posted.bot.clone(), to),
                    posted.message_id,
                    &posted.segments,
                )
            }),
            forwards: Forwards::of(
                posted.bot.clone(),
                Some(group.clone()),
                posted.message_id,
                &posted.segments,
            ),
        };
        let heard = Heard {
            id: message.id.clone(),
            number: message.number,
            venue,
            sender: external,
            name: posted.name.clone(),
            text: message.text.clone(),
            mentions_me: flags.mentions_me,
            quotes_me,
            mentions_others: !mentions.is_empty(),
            reply_to: posted.segments.reply_to.clone(),
            wordless: posted.segments.wordless(),
            media: posted
                .segments
                .media
                .iter()
                .map(|media| media.kind)
                .collect(),
        };
        // 聊天记录（设计 04，施工 C-1 上）：照记进会话的同一处记一份。
        self.record_heard(
            (platform, &heard.venue),
            &heard.sender,
            &posted,
            &heard.text,
        );
        // 问进站挂接点的那一条（施工 O-43 中，`onebot.md`「挂接点」第 3 条）。
        let ask = json!({
            "venue": heard.venue.id().as_str(),
            "bot": posted.bot,
            "msg": posted.message_id.to_string(),
            "sender": {"id": heard.sender.as_str(), "name": posted.name},
            "text": heard.text,
            "mentions_me": flags.mentions_me,
            "quotes_me": quotes_me,
            "media": inbound_media(&posted),
        });
        let note = applied::extra_prompt(&applied).map(str::to_string);
        let Some((session, reply)) = self.submit(message).await? else {
            return Ok(());
        };
        // 发言排行（施工 I-20）：核心收下了的才数。
        if crate::core::reason(&reply).is_none() {
            self.counters
                .spoke(heard.venue.id().as_str(), heard.sender.as_str());
        }
        // 群名、附加提示词放进 system（施工 O-49、O-58 上，`titled`）：判以前放好，这一条开的轮就带上。
        self.titled(&session, group.clone(), place.as_deref())
            .await?;
        self.noted(&session, note.as_deref()).await?;
        // 借了终端管理员、她自己的名字的记一块冒充提醒（施工 O-53，`impostor`）：判以前记好，这一条开的轮就看得到。
        if let Some(name) = posted.name.as_deref() {
            let who = (&heard.sender, name, posted.message_id);
            self.impostor(&session, (platform, group, posted.bot), who)
                .await?;
        }
        // 被拒的没有序号，不判。
        match reply["result"]["events"][0].as_u64() {
            Some(seq) => self.steer(platform, (session, seq), heard, ask).await,
            None => Ok(()),
        }
    }
}
