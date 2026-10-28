//! 好友请求、群邀请（施工 O-27，`onebot.md` 第一条「好友请求」；18 第十一节，2026-10-10 项目主人定）：白名单成员加她好友，
//! 经收到请求的那个机器人号现在的连接调 `set_friend_add_request {flag, approve: true}`；终端管理员也同意（施工 I-16 照旧版：问核心的
//! `venue.binding`，同 `revive::terminal_admin`）；别人的不回、放着（不拒）。群邀请只记一行，放着。
//!
//! 调 NapCat、等回应是一个另起的任务（`Route` 的 `chores`）：不卡跟核心的那一头。运行日志各记一行，不记验证消息、标记。

use std::sync::Arc;

use super::Route;
use crate::TARGET;
use crate::chat::{Bots, Platform};
use crate::core::Gone;

impl Route {
    /// 平台 `platform` 上机器人号 `bot` 收到号 `user` 加好友的请求，标记是 `flag`：`user` 在白名单里、是终端管理员的另起任务同意，别的记一行、放着。
    ///
    /// # Errors
    ///
    /// 问对应表时写不出去、核心断开。
    pub(super) async fn befriend(
        &mut self,
        platform: Platform,
        bot: i64,
        user: i64,
        flag: String,
    ) -> Result<(), Gone> {
        let Ok(who) = platform.person(user) else {
            return Ok(());
        };
        let known =
            self.whitelisted(&who) || self.terminal_admin(who.as_str()).await? == Some(true);
        if !known {
            tracing::info!(target: TARGET, bot, user, "friend request left pending");
            return Ok(());
        }
        self.chores
            .spawn(approve(self.platforms.bots(platform), bot, user, flag));
        Ok(())
    }
}

/// 机器人号 `bot` 收到号 `user` 邀请她进群 `group`：只记一行，放着（18 第十一节「群邀请只记下，保持待定」）。
pub(super) fn invited(bot: i64, group: i64, user: i64) {
    tracing::info!(target: TARGET, bot, group, user, "group invite left pending");
}

/// 经 `bots` 里号 `bot` 那时的连接同意 `user` 的好友请求 `flag`。没连着、NapCat 回失败、等不到的记一行 `WARN`。
async fn approve(bots: Arc<Bots>, bot: i64, user: i64, flag: String) {
    let Some(link) = bots.get(bot) else {
        tracing::warn!(target: TARGET, bot, user, "friend request not approved, bot not connected");
        return;
    };
    match link.befriend(&flag).await {
        Ok(_) => tracing::info!(target: TARGET, bot, user, "friend request approved"),
        Err(error) => {
            tracing::warn!(target: TARGET, bot, user, error = ?error, "friend request not approved");
        }
    }
}
