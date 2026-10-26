//! 她在这个场所叫什么（施工 I-38，核心 E-1修三）：场所消息的格带 `my_name`，核心拿它认出群里说的「Miyu」是她自己，不当成一个
//! 认识的人记。每条都带，取发这条时的值。
//!
//! - 群：她在这个群的群名片，空着的照 QQ 昵称（`get_group_member_info` 回的就是这样，[`Route::member`]）。记在群成员的缓存里，
//!   `group_card` 通知来了跟着改（[`Event::Renamed`](crate::chat::Event::Renamed)），过了缓存时长的再问。
//! - 私聊：她的 QQ 昵称（`get_login_info`），同样记一阵。
//! - 问不到的（没连着、失败、回的是空的）格里不写，核心照旧只认人格名；同一个场所缓存时长以内不再问，免得每条消息都等一次时限。

use std::time::Instant;

use super::Route;
use super::fields::{NAME, clean};
use crate::TARGET;

impl Route {
    /// 机器人号 `bot` 在群 `group`（私聊的没有）里叫什么，洗过；问不到的是空的（见模块开头）。
    pub(super) async fn my_name(&mut self, bot: i64, group: Option<i64>) -> Option<String> {
        let now = Instant::now();
        let Some(group) = group else {
            if let Some(name) = self.members.nickname(bot, now) {
                return clean(name, NAME);
            }
            if !self.members.ask_due(bot, None, now) {
                return None;
            }
            let link = self.bots.get(bot)?;
            let name = match link.nickname().await {
                Ok(name) => name?,
                Err(error) => {
                    tracing::debug!(target: TARGET, bot, error = ?error, "own nickname unknown");
                    return None;
                }
            };
            let cleaned = clean(&name, NAME);
            self.members.nicknamed(bot, name, Instant::now());
            return cleaned;
        };
        let mut asking = self.members.name(group, bot, now).is_some()
            || self.members.ask_due(bot, Some(group), now);
        self.member(group, bot, bot, &mut asking).await
    }

    /// 群 `group` 里号是 `user` 的人的群名片改成了 `card`（清掉了的是空的）：缓存里记新的；清掉的忘掉，下回问到的是昵称。
    pub(super) fn renamed(&mut self, group: i64, user: i64, card: Option<String>) {
        match card.filter(|card| !card.trim().is_empty()) {
            Some(card) => self.members.remember(group, user, card, Instant::now()),
            None => self.members.forget(group, user),
        }
    }
}
