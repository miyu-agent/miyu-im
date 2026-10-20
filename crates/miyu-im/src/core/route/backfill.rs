//! 补拉回来的群消息（施工 I-15，设计 18 第十一节「补拉消息」）：桥断线时漏掉、重连以后驱动从平台补回来的（`Event::Backfilled`），
//! 照群消息的样子记进这个群的场所会话，只进聊天记录。
//!
//! - 场所的格多两格：`backfill`（核心要它同时带 `ambient`）、`sent`（平台给的时刻，`history` 照它显示、照它筛）。核心照这两格
//!   不把它收进群聊近况、判官看的记录，不开回合。
//! - 命令编号同实时收到的（平台、账号、消息编号、平台时刻）：补回来的和实时收到过的撞了核心只记一次。
//! - 不当斜杠命令、不判、不取图、不展开转发、不问挂接点，今天的数不加（统计页的「收到」只算实时的）。

use miyu_chat::VenueKind;
use miyu_client::time::Timestamp;
use serde_json::json;

use super::fields::{Flags, fields};
use super::session::{Place, SEND};
use super::{Message, Route, applied};
use crate::TARGET;
use crate::chat::Posted;
use crate::core::{Gone, reason};

/// 平台给的时刻（整数秒）写成核心认的样子；没带的（0）、写不出的是空的。
pub(super) fn sent(seconds: i64) -> Option<String> {
    let millis = seconds.checked_mul(1000).filter(|&millis| millis > 0)?;
    Some(Timestamp::from_unix_millis(millis)?.to_string())
}

impl Route {
    /// 群号是 `group` 的群里补拉回来的一条消息 `posted`。
    pub(super) async fn backfilled(&mut self, group: i64, posted: Posted) -> Result<(), Gone> {
        let made = self
            .platform
            .venue(VenueKind::Group, group)
            .and_then(|venue| Ok((venue, self.platform.person(posted.user)?)));
        let (venue, external) = match made {
            Ok(made) => made,
            Err(error) => {
                tracing::warn!(target: TARGET, group, message = posted.message_id, error = %error, "venue not made, backfill dropped");
                return Ok(());
            }
        };
        let applied = self.applied(&venue);
        let text = self.named(group, &posted).await;
        if text.trim().is_empty() && posted.segments.media.is_empty() {
            return Ok(());
        }
        let at = posted.segments.at();
        let mentions: Vec<_> = at
            .iter()
            .filter(|user| **user != posted.bot)
            .filter_map(|user| self.platform.person(*user).ok())
            .collect();
        let flags = Flags {
            mentions_me: at.contains(&posted.bot),
            ambient: true,
            asleep: false,
            show_ids: applied::show_ids(&applied),
        };
        let mut venue_fields = fields(&posted, &mentions, flags);
        venue_fields["backfill"] = json!(true);
        if let Some(sent) = sent(posted.time) {
            venue_fields["sent"] = json!(sent);
        }
        let message = Message {
            id: self
                .platform
                .command_id(posted.bot, posted.message_id, posted.time),
            number: posted.message_id,
            acting: json!({"external": external, "role": applied::role(&applied, &external)}),
            fields: venue_fields,
            place: Place::group(&venue, &applied, posted.bot, group),
            text,
            pictures: None,
            forwards: None,
        };
        let Some((_, reply)) = self.deliver(&message, SEND, &[]).await? else {
            return Ok(());
        };
        match reason(&reply) {
            None => {
                tracing::info!(target: TARGET, venue = %venue.id(), message = posted.message_id, "message backfilled")
            }
            Some(other) => {
                tracing::warn!(target: TARGET, venue = %venue.id(), message = posted.message_id, reason = other, "backfill refused");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
