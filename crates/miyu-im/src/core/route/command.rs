//! 斜杠命令（施工 O-19，`onebot.md` 第一条「怎么走」第 7、8 条之间的「斜杠命令」，`venues.md`「斜杠命令」）：私聊、群（施工
//! O-22，「群消息」第 7 条）的文字去掉开头的空白以后以 `/` 开头的，先原样交核心的 `command.run`，和发消息同一个命令编号。
//! 核心认命令、判谁能用、执行、照连接的语言写好回执；桥只分三种回应：成了的回执发回去，核心认不出的（`unknown_command`）
//! 交回给 `Route::submit` 照普通的话发，别的被拒把核心拒绝时那一句（`error.message`，照连接的语言写好的，「施工时定的」
//! 第 31 条）发回去（群里的发回群里）、不交给她。发回去的交 `receipt`：入队（施工 O-25 中），群里的过几秒撤回（施工 O-25 上）。
//! 运行日志只记正名和原因码，不记原文。
//!
//! 施工 O-47（2026-10-17 项目主人定：QQ 里只有管理员跑命令时才出现系统口吻的话）：被拒的那一句只回给终端管理员和场所规则
//! `managers` 里的人，别人被拒什么都不发；群里成了的回执照 `im.recall_receipts` 撤不撤（出厂只撤 `clear`、
//! `clear-memory`），被拒的那一句照旧撤。
//!
//! 施工 I-55（2026-10-27 项目主人：群里一直有人说话，她总在一轮里，`/clear` 就一直被拒）：被拒是因为这一轮还在进行的
//! （`turn_running`：`/clear`、`/compact`、`/undo` 这些），先打断这个会话的那一轮（排着的留着），再跑一次。核心判过谁能用
//! 以后才拒这个，所以不另查是谁打的。

use miyu_client::ids::ExternalId;
use serde_json::{Value, json};

use super::{Message, Route};
use crate::TARGET;
use crate::core::{Gone, reason};

/// `as` 的 `role`：场所规则 `managers` 里的人（「群消息」第 4 条）。
const MANAGER: &str = "manager";

/// 打断以后再跑的那一次的命令编号：这条消息的加上它（同一个编号核心只算一次，施工 I-55）。
const FORCED: &str = ":forced";

mod model;

impl Route {
    /// 把 `message` 当斜杠命令交给核心（「斜杠命令」第 1 到 5 条）。办完了（成了、被拒、不接、找不到会话）交回真；核心认不
    /// 出的交回假，由用的一方照普通的话发。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn command(&mut self, message: &Message) -> Result<bool, Gone> {
        // `/model` 桥自己办（施工 I-53）；不归它管的人打的照原来交核心。
        if let Some(query) = model::model_query(&message.text)
            && self.model_command(message, query).await?
        {
            return Ok(true);
        }
        let Some((session, reply)) = self.deliver(message, "command.run", &[]).await? else {
            return Ok(true);
        };
        let reply = match reason(&reply) {
            Some("turn_running") => self.forced(message, &session, reply).await?,
            _ => reply,
        };
        let (venue, number) = (&message.place.peer.venue, message.number);
        let (said, recall) = match reason(&reply) {
            None => {
                let command = reply["result"]["command"].as_str().unwrap_or_default();
                tracing::info!(target: TARGET, venue = %venue, message = number, command, "command ran");
                let recall = self.recalled.iter().any(|name| name == command);
                (said(&reply["result"]["said"]).to_string(), recall)
            }
            Some(reason @ "unknown_command") => {
                tracing::debug!(target: TARGET, venue = %venue, message = number, reason, "not a command");
                return Ok(false);
            }
            Some(reason) => {
                tracing::info!(target: TARGET, venue = %venue, message = number, reason, "command refused");
                if !self.hears_refusals(message).await? {
                    return Ok(true);
                }
                (said(&reply["error"]["message"]).to_string(), true)
            }
        };
        self.receipt(&session, &said, recall).await?;
        Ok(true)
    }

    /// 命令因为会话 `session` 这一轮还在进行被拒了（回应 `refused`，施工 I-55）：打断那一轮（排着的留着），换一个命令编号再跑
    /// 一次，交回这一次的回应。打断不了的（没在进行的除外）交回原来的。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    async fn forced(
        &mut self,
        message: &Message,
        session: &str,
        refused: Value,
    ) -> Result<Value, Gone> {
        let (venue, number) = (&message.place.peer.venue, message.number);
        let interrupt = json!({"session": session, "queued": "keep"});
        let reply = self.core.call("session.interrupt", interrupt).await?;
        if let Some(reason) = reason(&reply).filter(|reason| *reason != "not_running") {
            tracing::info!(target: TARGET, venue = %venue, message = number, reason, "turn not interrupted for a command");
            return Ok(refused);
        }
        tracing::info!(target: TARGET, venue = %venue, message = number, "turn interrupted for a command");
        let params = json!({"session": session, "text": message.text, "as": message.acting});
        let id = format!("{}{FORCED}", message.id);
        self.core.call_as(&id, "command.run", params).await
    }

    /// 打命令的人听不听得到被拒的那一句（施工 O-47）：场所规则 `managers` 里的（`as.role` 是 `manager`）、终端管理员听得到，
    /// 别人听不到。终端管理员：私聊接进来、对方又不在白名单里的只会是他（陌生人的私聊不接）；别的问到的照记的，没记着的问核心的
    /// `venue.binding`；问不到的（核心拒了）群里照他在这个群说过的话带不带 `account` 兜底（同「平台工具（一）」第 3 条）。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    async fn hears_refusals(&mut self, message: &Message) -> Result<bool, Gone> {
        if message.acting["role"] == MANAGER {
            return Ok(true);
        }
        let place = &message.place;
        if place.private_chat() && !place.listed() {
            return Ok(true);
        }
        let Some(who) = message.acting["external"].as_str() else {
            return Ok(false);
        };
        if self.terminal_admin(who).await? == Some(true) {
            return Ok(true);
        }
        let Ok(id) = ExternalId::parse(who) else {
            return Ok(false);
        };
        Ok(self
            .venues
            .get(place.peer.venue.as_str())
            .and_then(|session| self.groups.get(session))
            .is_some_and(|group| group.is_admin(&id)))
    }
}

/// 像不像一条斜杠命令：去掉开头的空白（Unicode 的空白，和核心的认法一样）以后以 `/` 开头。是哪个命令由核心认（「施工时
/// 定的」第 32 条）。
pub(super) fn looks_like(text: &str) -> bool {
    text.trim_start().starts_with('/')
}

/// 回应里给人看的那一句；不是字的（照说不会）当空的，空的不发。
fn said(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}
