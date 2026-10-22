//! 退信（施工 O-25 下，`onebot.md`「退信」）：没成的一段记 `ext.im.venues.failed`，她的话再经核心的 `session.note` 给那个会话记一块
//! `undelivered` 事实。她一句话拆成的几段（I-26）接连没成的只退一块，写着头一段的开头：退过的那一句记在 `Route` 的 `bounced`，
//! 这个会话发成了一段就清掉。

use std::collections::BTreeMap;

use serde_json::json;

use super::{FAILED, HEAD, Item, Piece, Route, UNDELIVERED, What, said};
use crate::TARGET;
use crate::core::{Gone, reason};
use crate::counters::Count;

/// 一段是她哪一句话的：她的话是那条线和回合编号，别的是空的。
fn sentence(piece: &Piece) -> Option<(String, u64)> {
    match &piece.what {
        What::Reply { line, turn, .. } => Some((line.clone(), *turn)),
        _ => None,
    }
}

impl Route {
    /// 没成的一段记 `failed {queued, why, detail?}`（第 4 条），运行日志记一行（不记原文）；她的话再退信（施工 O-25 下，「退信」）。
    /// 她一句话拆成的几段（带停顿的），同一句前一段刚退过信的只记 `failed`。
    pub(super) async fn failed(
        &mut self,
        session: &str,
        item: &Item,
        (why, detail): (&str, Option<String>),
    ) -> Result<(), Gone> {
        let venue = self
            .peers
            .get(session)
            .map_or_else(String::new, |peer| peer.venue.to_string());
        let chars = item.piece.text.chars().count();
        tracing::warn!(target: TARGET, venue = %venue, why, chars, "reply not sent");
        if said(&item.piece.what) {
            self.counters.bump(Count::Undelivered, &venue);
        }
        let mut body = json!({"queued": item.queued, "why": why});
        if let Some(detail) = &detail {
            body["detail"] = json!(detail);
        }
        self.append(session, None, FAILED, body).await?;
        match item.piece.what {
            What::Reply { ref line, .. } => {
                let sentence = sentence(&item.piece);
                if !item.piece.pause.is_zero() && self.bounced.get(session) == sentence.as_ref() {
                    return Ok(());
                }
                if let Some(sentence) = sentence {
                    self.bounced.insert(session.to_string(), sentence);
                }
                let detail = detail.unwrap_or_default();
                self.undelivered((session, line), &venue, item, (why, &detail))
                    .await
            }
            What::Image(ref image) => {
                let detail = detail.unwrap_or_default();
                self.unsent_image(image, (why, &detail)).await
            }
            // 长文的第一页照她的话退信，后面几页照发图记 `unshown`（施工 O-59 上）。
            What::Page(ref page) if page.first => {
                let detail = detail.unwrap_or_default();
                self.undelivered((session, &page.image.line), &venue, item, (why, &detail))
                    .await
            }
            What::Page(ref page) => {
                let detail = detail.unwrap_or_default();
                self.unsent_image(&page.image, (why, &detail)).await
            }
            What::Receipt { .. } => Ok(()),
        }
    }

    /// 她的一段 `item` 没发出去（为什么 `why`、NapCat 说的 `detail`，没有的是空的）：照退信的模板写一块，经核心的 `session.note`
    /// 记进她说它的那条线 `line`，命令编号是入队那一条的序号加 `/note`（「退信」第 2、3 条）；那条线是支线、已经封存了的
    /// （`session_sealed`）改记进会话 `session`（主线，施工 O-36）。`venue` 只记运行日志。核心拒了的（照说不会）记一行，不再试。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    async fn undelivered(
        &mut self,
        (session, line): (&str, &str),
        venue: &str,
        item: &Item,
        (why, detail): (&str, &str),
    ) -> Result<(), Gone> {
        let head: String = item.piece.text.trim().chars().take(HEAD).collect();
        let fields = BTreeMap::from([("why", why), ("detail", detail), ("text", head.as_str())]);
        let text = match self.rules.undelivered().render(&fields) {
            Ok(text) => text,
            Err(error) => {
                // 照说不会：起来时查过模板只要这三个字段（`rules/facts.rs`）。
                tracing::warn!(target: TARGET, venue, error = %error, "undelivered not written");
                return Ok(());
            }
        };
        let id = format!("{}/note", item.queued);
        let facts = json!([{"kind": UNDELIVERED, "text": text}]);
        let params = json!({"session": line, "facts": facts});
        let mut reply = self.core.call_as(&id, "session.note", params).await?;
        if line != session && reason(&reply) == Some("session_sealed") {
            let params = json!({"session": session, "facts": facts});
            reply = self.core.call_as(&id, "session.note", params).await?;
        }
        match reason(&reply) {
            None => tracing::info!(target: TARGET, venue, why, "undelivered noted"),
            Some(reason) => tracing::warn!(target: TARGET, venue, reason, "note refused"),
        }
        Ok(())
    }
}
