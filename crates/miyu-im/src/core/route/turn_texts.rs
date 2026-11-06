//! 群里每开一轮时 im 自己加的两样字（从 `lanes.rs` 挪出来，施工 I-172）：会话的说话方式那一块事实（施工 I-140，卡片上写了才有，
//! 排在这一轮的事实最前面），每一轮最末尾的尾巴（施工 I-172，核心 RT-1 的 `tail`：别照历史里的话、包括她自己的长回复学说话，
//! 不进历史）。都只有群会话有，字是出厂的模板（`facts/style-note.txt`、`tail/context-style.txt`），登记在 `docs/ledger.md`。

use miyu_chat::{Venue, VenueKind};
use miyu_client::ids::VenueId;
use serde_json::{Value, json};

use super::{Route, applied};
use crate::TARGET;

/// 会话的说话方式那一块事实的 `kind`（施工 I-140）。
const STYLE_NOTE: &str = "style-note";

impl Route {
    /// 群会话每一轮的尾巴（施工 I-172）：场所 `venue` 是群的才有，照出厂的那一句；写不出的（照说不会：起来时查过）记一行、不带。
    pub(super) fn context_style_tail(&self, venue: &str) -> Option<String> {
        let venue = VenueId::parse(venue)
            .ok()
            .and_then(|id| Venue::parse(&id))?;
        if venue.kind() != VenueKind::Group {
            return None;
        }
        let fields: [(&str, &str); 0] = [];
        match self
            .rules
            .context_style()
            .render(&fields.iter().copied().collect())
        {
            Ok(text) => Some(text.trim_end().to_string()),
            Err(error) => {
                tracing::warn!(target: TARGET, error = %error, "context style tail not written");
                None
            }
        }
    }

    /// 会话的说话方式那一块事实（施工 I-140）：场所 `venue` 是群、卡片上写了「这个会话的说话方式」的才有，写的那段照模板转义填进去；模板
    /// 写不出的（照说不会：起来时查过字段）记一行、不带。
    pub(super) fn style_note_fact(&mut self, venue: &str) -> Option<Value> {
        let venue = VenueId::parse(venue)
            .ok()
            .and_then(|id| Venue::parse(&id))?;
        if venue.kind() != VenueKind::Group {
            return None;
        }
        let applied = self.applied(&venue);
        let note = applied::style_note(&applied)?;
        match self
            .rules
            .style_note()
            .render(&[("text", note)].into_iter().collect())
        {
            Ok(text) => Some(json!({"kind": STYLE_NOTE, "text": text})),
            Err(error) => {
                tracing::warn!(target: TARGET, error = %error, "style note not written");
                None
            }
        }
    }
}
