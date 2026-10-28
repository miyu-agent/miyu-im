//! 插件记一句给 AI（施工 I-8 上，设计 01 第四节第 6 条）：im 的插件经核心的 `package.call` 调 im 导出的 `chat.note {venue, kind,
//! text}`。场所会话归 im 的系统账号，插件写不进去，所以由 im 记：套上外壳（模板 `facts/plugin-note.txt`，`<种类>字</种类>`，字照
//! 转义，给模型看的字，登记在 `docs/ledger.md`），经核心的 `session.note` 记进那个场所的主会话，不开回合；这一轮在跑的下一次请求
//! 看到，空闲的下一轮开头看到。
//!
//! - 只收在那个场所开着的插件调的（`Hooks::allowed`），别的拒 `not_allowed`。
//! - `kind` 是小写字母开头、只有小写字母、数字、`-`，最多 32 个字（当标签名）；`text` 去掉首尾空白不空、最多 [`TEXT`] 个字：
//!   不合的拒 `bad_params`。场所写错的也是。
//! - 那个场所还没有会话的拒 `no_session`；核心拒了的照它的原因拒。成了答 `{"noted": true}`。

use std::collections::BTreeMap;

use serde_json::{Value, json};

use miyu_chat::Venue;
use miyu_client::ids::VenueId;

use miyu_chat::VenueKind;

use super::Route;
use super::session::Place;
use crate::TARGET;
use crate::chat::{Platform, To};
use crate::core::{Gone, reason};

/// 一句最多几个字。
const TEXT: usize = 500;

/// 种类最多几个字。
const KIND: usize = 32;

/// 种类的写法：小写字母开头，只有小写字母、数字、`-`。
fn kind_ok(kind: &str) -> bool {
    let mut chars = kind.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && kind.chars().count() <= KIND
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// 读 `chat.note` 的参数：场所、种类、去掉首尾空白的字。写错的是空的。
fn read(args: &Value) -> Option<(Venue, &str, &str)> {
    let venue = VenueId::parse(args["venue"].as_str()?).ok()?;
    let venue = Venue::parse(&venue)?;
    let kind = args["kind"].as_str().filter(|kind| kind_ok(kind))?;
    let text = args["text"]
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty() && text.chars().count() <= TEXT)?;
    Some((venue, kind, text))
}

impl Route {
    /// 核心转来的一次 `chat.note`（`pushed`）：照模块的说明记、答。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn note_exported(&mut self, pushed: &Value) -> Result<(), Gone> {
        let id = pushed["id"].clone();
        let from = pushed["params"]["from"].as_str().unwrap_or_default();
        let answerer = self.core.answerer();
        let Some((venue, kind, text)) = read(&pushed["params"]["params"]) else {
            tracing::info!(target: TARGET, from, why = "bad_params", "plugin note refused");
            answerer.refuse(&id, "bad_params");
            return Ok(());
        };
        // 平台照场所编号的第一段（施工 I-70 上）。
        let at = self
            .platforms
            .of_venue(&venue)
            .and_then(|platform| Some((platform, platform.to_of(&venue)?)));
        if !at.is_some_and(|(platform, to)| self.hooks.allowed(from, platform, Some(to))) {
            tracing::info!(target: TARGET, from, venue = %venue.id(), why = "not_allowed", "plugin note refused");
            answerer.refuse(&id, "not_allowed");
            return Ok(());
        }
        let Some(session) = self.venues.get(venue.id().as_str()).cloned() else {
            tracing::info!(target: TARGET, from, venue = %venue.id(), why = "no_session", "plugin note refused");
            answerer.refuse(&id, "no_session");
            return Ok(());
        };
        match self.note_in(&session, kind, text).await? {
            None => {
                tracing::info!(target: TARGET, from, venue = %venue.id(), kind, "plugin noted");
                answerer.answer(&id, json!({"noted": true}));
            }
            Some(why) => {
                tracing::warn!(target: TARGET, from, venue = %venue.id(), why, "plugin note refused by core");
                answerer.refuse(&id, &why);
            }
        }
        Ok(())
    }

    /// 平台上的机器人号 `bot` 收进的、驱动带的插件给场所 `to` 记一句（施工 Q-3，`Event::Noted`）：同插件的外壳，记进那个群的主会话（还没有的先找、建，同她被禁言
    /// 的事实，`muted.rs`）。私聊的、记不进的记一行运行日志。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn note_built_in(
        &mut self,
        (platform, bot): (Platform, i64),
        to: To,
        kind: &str,
        text: &str,
    ) -> Result<(), Gone> {
        let To::Group(group) = to else {
            tracing::warn!(target: TARGET, kind, "built-in note for a private chat dropped");
            return Ok(());
        };
        let Ok(venue) = platform.venue(VenueKind::Group, group) else {
            return Ok(());
        };
        let place = Place::group((platform, &venue), &self.applied(&venue), bot, group);
        if !self.found(&place).await? {
            tracing::warn!(target: TARGET, group, kind, "built-in note dropped, no session");
            return Ok(());
        }
        let Some(session) = self.venues.get(venue.id().as_str()).cloned() else {
            return Ok(());
        };
        match self.note_in(&session, kind, text).await? {
            None => tracing::info!(target: TARGET, venue = %venue.id(), kind, "built-in noted"),
            Some(why) => {
                tracing::warn!(target: TARGET, venue = %venue.id(), kind, why, "built-in note refused");
            }
        }
        Ok(())
    }

    /// 套上外壳记进会话 `session`：成了的是空的，核心拒了的交回原因。
    async fn note_in(
        &mut self,
        session: &str,
        kind: &str,
        text: &str,
    ) -> Result<Option<String>, Gone> {
        let fields = BTreeMap::from([("kind", kind), ("text", text)]);
        let Ok(rendered) = self.rules.plugin_note().render(&fields) else {
            // 照说不会：起来时查过模板只要这两个字段（`rules/facts.rs`）。
            return Ok(Some("internal".to_string()));
        };
        let facts = json!([{"kind": kind, "text": rendered}]);
        let reply = self
            .core
            .call("session.note", json!({"session": session, "facts": facts}))
            .await?;
        Ok(reason(&reply).map(str::to_string))
    }
}

#[cfg(test)]
mod tests;
