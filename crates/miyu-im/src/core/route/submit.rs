//! 交一条人话进会话（原来在 `route.rs`，施工 I-231 为行数上限挪出来，一字未改）：斜杠命令、展开转发、带图，再照 `session.send` 交。

use serde_json::Value;

use super::{Message, Route, command, fields, session};
use crate::TARGET;
use crate::core::{Gone, reason};
use crate::counters::Count;

impl Route {
    /// 交一条消息：`/` 开头的先当斜杠命令交（「斜杠命令」），办完了的不再发；别的照 `session.send` 发（第 8 条，「群消息」
    /// 第 6 条），交回会话编号和回应（接受的、拒绝的都原样：群的照它判，施工 O-23）。要带图的（施工 O-33）、要展开转发的（施工
    /// O-34）先找到会话（不接的不去取），取了图带上、展开的转发写进场所的格；私聊没有字、没带东西、图又一张都没取到的不送。命令办完了的、
    /// 不接的、找不到会话的、不送的是空的。
    pub(super) async fn submit(
        &mut self,
        mut message: Message,
    ) -> Result<Option<(String, Value)>, Gone> {
        if command::looks_like(&message.text) && self.command(&message).await? {
            return Ok(None);
        }
        if !self.expanded(&mut message).await? {
            return Ok(None);
        }
        let mut attachments = Vec::new();
        if let Some(pictures) = message.pictures.as_ref().filter(|pictures| pictures.any()) {
            if !self.found(&message.place).await? {
                return Ok(None);
            }
            let venue = message.place.peer.venue.to_string();
            let shown;
            let platform = message.place.peer.platform;
            (attachments, shown) = self.pictures((platform, pictures), &venue).await?;
            // 私聊里交了原图的那几样记 `shown`（施工 O-35）；群里冲她来的这一步不记。
            if message.place.private_chat() {
                fields::shown(&mut message.fields, &shown);
            }
        }
        let bare = message.text.trim().is_empty() && message.fields.get("media").is_none();
        if message.place.private_chat() && bare && attachments.is_empty() {
            tracing::debug!(target: TARGET, venue = %message.place.peer.venue, message = message.number, "nothing to send");
            return Ok(None);
        }
        let sent = self.deliver(&message, session::SEND, &attachments).await?;
        let Some((session, reply)) = sent else {
            return Ok(None);
        };
        let (venue, number) = (&message.place.peer.venue, message.number);
        match reason(&reply) {
            None => {
                let chars = message.text.chars().count();
                tracing::info!(target: TARGET, venue = %venue, message = number, chars, "message sent in");
                self.counters.bump(Count::Received, &venue.to_string());
            }
            Some(other) => {
                tracing::warn!(target: TARGET, venue = %venue, message = number, reason = other, "message refused");
            }
        }
        Ok(Some((session, reply)))
    }
}
