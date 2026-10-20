//! 驱动交的工具的调用（施工 T-1，`crate::toolset`）：读的一头把它们的 `tool.call` 照推送交到这里。照会话填好场景（发到哪、机器人
//! 号、谁叫的、工作区、那个机器人号的连接）交给驱动：当场答的写回去；要去平台上办的另起一个任务（`chores`，同取图）等它办完再
//! 答，跟核心的那一头不等。桥不认识这个会话的答 `unreachable`。

use std::sync::Arc;

use serde_json::Value;

use super::Route;
use crate::TARGET;
use crate::chat::To;
use crate::core::Gone;
use crate::toolset::{Answers, Call, Planned, ToolSet};

impl Route {
    /// 核心转来的一次驱动的工具的调用（`pushed` 是 `tool.call`），那一套是 `set`、答的话是 `answers`。
    ///
    /// # Errors
    ///
    /// 收这个会话留着的推送时写不出去、核心断开。
    pub(super) async fn driven_called(
        &mut self,
        pushed: &Value,
        (set, answers): (Arc<dyn ToolSet>, Arc<Answers>),
    ) -> Result<(), Gone> {
        let params = &pushed["params"];
        let (id, tool) = (
            pushed["id"].clone(),
            params["tool"].as_str().unwrap_or_default(),
        );
        let line = params["session"].as_str().unwrap_or_default().to_string();
        let session = self.main_of(&line);
        let answerer = self.core.answerer();
        self.gather(&session).await?;
        let Some(peer) = self.peers.get(&session).cloned() else {
            tracing::warn!(target: TARGET, session, tool, why = "unreachable", "platform tool failed");
            answerer.answer(&id, self.tools.answer("unreachable", &[], true));
            return Ok(());
        };
        let caller = match peer.to {
            To::Private(user) => Some(user),
            To::Group(_) => caller_number(params),
        };
        let call = Call {
            tool: tool.to_string(),
            args: params["args"].clone(),
            venue: peer.venue.as_str().to_string(),
            to: peer.to,
            bot: peer.bot,
            caller,
            cwd: params["cwd"].as_str().unwrap_or_default().to_string(),
            link: self.bots.get(peer.bot),
            answers,
        };
        let (venue, tool) = (call.venue.clone(), call.tool.clone());
        match set.plan(call) {
            Planned::Now(answer) => {
                logged(&venue, &tool, &answer);
                answerer.answer(&id, answer);
            }
            Planned::Later(pending) => {
                self.chores.spawn(async move {
                    let answer = pending.await;
                    logged(&venue, &tool, &answer);
                    answerer.answer(&id, answer);
                });
            }
        }
        Ok(())
    }
}

/// 记一行运行日志：出错的 `WARN platform tool failed`，成了的 `INFO platform tool done`。
fn logged(venue: &str, tool: &str, answer: &Value) {
    if answer["error"] == true {
        tracing::warn!(target: TARGET, venue, tool, "platform tool failed");
    } else {
        tracing::info!(target: TARGET, venue, tool, "platform tool done");
    }
}

/// 叫她的人的号：`tool.call` 的 `by` 是外部身份的照 `by.id`，私聊里认出的终端管理员照 `by.via`；别的是空的。
fn caller_number(params: &Value) -> Option<i64> {
    let by = &params["by"];
    let id = match by["kind"].as_str() {
        Some("external") => by["id"].as_str(),
        Some("person") => by["via"].as_str(),
        _ => None,
    }?;
    id.rsplit_once(':')?.1.parse().ok()
}
