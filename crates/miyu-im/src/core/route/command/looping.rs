//! 群里的 `/loop`（施工 I-239，项目主人 11-14 定）：群里只当「自主循环」（卡片上的 `glance`，设计 05 的「看一眼」）的开关，
//! 桥自己办，不交核心。私聊照旧交核心（核心的循环）。
//!
//! - `/loop`：这个群的卡片打开自主循环；`/loop off`：关上。改卡片同 `/model`（[`super::picking`]：这个号的那张，没有的
//!   改整个平台的那张，都没有的建一张这个号的）。
//! - 带间隔、带话的（`/loop 5m …`、`/loop 看看群里`）群里不要：回一句用法。
//! - 谁能用同 `/model` 这几条：终端管理员和场所规则 `managers` 里的人（`hears_refusals`）；别人打的什么都不回，也不交核心。
//! - 回的那一句照回执发（不撤回），字照连接的语言，在出厂的 `cards/human/page.json`（`loop-*`）。

use serde_json::json;

use super::super::{Message, Route};
use super::picking::Saved;
use crate::TARGET;
use crate::chat::To;
use crate::core::Gone;

/// 命令名。
const LOOP: &str = "/loop";

/// 卡片上自主循环那一格。
const GLANCE: &str = "glance";

/// 群里的 `/loop` 说的是哪一种。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Asked {
    /// 光打 `/loop`：开。
    On,
    /// `/loop off`：关。
    Off,
    /// 别的写法：回用法。
    Other,
}

/// `text` 是不是 `/loop` 这条命令、说的哪一种；`/looping` 这类不是。
pub(super) fn asked(text: &str) -> Option<Asked> {
    let rest = text.trim().strip_prefix(LOOP)?;
    if rest
        .chars()
        .next()
        .is_some_and(|first| !first.is_whitespace())
    {
        return None;
    }
    Some(match rest.trim() {
        "" => Asked::On,
        off if off.eq_ignore_ascii_case("off") => Asked::Off,
        _ => Asked::Other,
    })
}

/// 群里打的才算（私聊照旧交核心）。
pub(super) fn in_group(message: &Message) -> bool {
    matches!(message.place.peer.to, To::Group(_))
}

impl Route {
    /// 办群里的 `/loop`（`asked` 是哪一种）。办完了交回真（不交核心）。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn loop_command(
        &mut self,
        message: &Message,
        asked: Asked,
    ) -> Result<bool, Gone> {
        let venue = &message.place.peer.venue;
        if !self.hears_refusals(message).await? {
            tracing::info!(target: TARGET, venue = %venue, message = message.number, command = "loop", reason = "not_manager", "command refused");
            return Ok(true);
        }
        let Some(session) = self.session_of(message).await? else {
            return Ok(true);
        };
        let words = self.rules.page_words().clone();
        let lang = self.core.language.clone();
        let word = |name: &str| words.get(&format!("loop-{name}"), &lang);
        let said = match asked {
            Asked::Other => word("usage"),
            Asked::On | Asked::Off => {
                let on = asked == Asked::On;
                match self.save_card_value(message, GLANCE, json!(on)) {
                    Saved::Done => word(if on { "on" } else { "off" }),
                    Saved::Refused(detail) => word("refused").replace("{detail}", &detail),
                    Saved::Failed => word("no-card"),
                }
            }
        };
        tracing::info!(target: TARGET, venue = %venue, message = message.number, command = "loop", "command ran");
        self.receipt(&session, &said, false).await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
