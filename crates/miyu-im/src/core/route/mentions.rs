//! @ 指定的人（施工 O-55，`onebot.md` 第一条「@ 指定的人」，2026-10-17 项目主人定，照旧版的 `qq_mention_users`）：群里她调
//! `mention {users}`，桥照号一个个认是不是这个群的成员（群成员缓存，没有的问 `get_group_member_info`），都是的记下：她在这条线
//! 这一轮的下一句话第一段前面 @ 他们（`say` 用掉）；有不是的整次不记，答哪几个不是。
//!
//! 记在桥的内存里，按线：换了一轮的、她这一轮没再说话的作废（下一次照新的一轮比）。

use std::collections::HashMap;

use serde_json::Value;

use super::Route;
use crate::TARGET;
use crate::chat::To;
use crate::core::Gone;

/// 她要 @ 的：按线（主线、支线的会话编号）记（回合编号，号）。
#[derive(Debug, Default)]
pub(super) struct Mentions(HashMap<String, (u64, Vec<String>)>);

/// 读 `mention` 的参数 `args` 里的号（纯逻辑）：`users` 是正整数（写成字的也认）的列表，去重、照先后，1 到 [`MOST`] 个；别的是空的。
pub(super) fn users(args: &Value) -> Option<Vec<i64>> {
    let list = args["users"].as_array()?;
    let mut found: Vec<i64> = Vec::new();
    for one in list {
        let number = match one {
            Value::Number(number) => number.as_i64(),
            Value::String(text) => text.trim().parse().ok(),
            _ => None,
        }
        .filter(|number| *number > 0)?;
        if !found.contains(&number) {
            found.push(number);
        }
    }
    (1..=MOST).contains(&found.len()).then_some(found)
}

/// 一次最多 @ 几个人（一段话前面挂太多 @ 就是刷屏）。
const MOST: usize = 10;

impl Route {
    /// 线 `line` 第 `turn` 轮她要 @ 的号：用掉；记的是别的一轮的作废，交回空的。
    pub(super) fn take_mentions(&mut self, line: &str, turn: u64) -> Vec<String> {
        match self.mentions.0.remove(line) {
            Some((kept, users)) if kept == turn => users,
            _ => Vec::new(),
        }
    }

    /// 核心转来的一次 `mention`（`pushed` 是 `tool.call`）：认号，都是这个群的成员的记下、答 `mentioned`，有不是的答
    /// `not-members`，写错的、群外调的答 `no-target`。
    ///
    /// # Errors
    ///
    /// 收这个会话留着的推送时写不出去、核心断开。
    pub(super) async fn mention_called(&mut self, pushed: &Value) -> Result<(), Gone> {
        let params = &pushed["params"];
        let (id, tool) = (&pushed["id"], params["tool"].as_str().unwrap_or_default());
        let line = params["session"].as_str().unwrap_or_default().to_string();
        let session = self.main_of(&line);
        let answerer = self.core.answerer();
        self.gather(&session).await?;
        let Some(peer) = self.peers.get(&session).cloned() else {
            tracing::warn!(target: TARGET, session, tool, why = "unreachable", "platform tool failed");
            answerer.answer(id, self.tools.answer("unreachable", &[], true));
            return Ok(());
        };
        let (To::Group(group), Some(users)) = (peer.to, users(&params["args"])) else {
            tracing::info!(target: TARGET, venue = %peer.venue, tool, why = "no-target", "platform tool refused");
            answerer.answer(id, self.tools.answer("no-target", &[], true));
            return Ok(());
        };
        let mut names = Vec::new();
        let mut strangers = Vec::new();
        let mut asking = true;
        for user in &users {
            match self.member(group, *user, peer.bot, &mut asking).await {
                Some(name) => names.push(name),
                None => strangers.push(user.to_string()),
            }
        }
        if !strangers.is_empty() {
            tracing::info!(target: TARGET, venue = %peer.venue, tool, why = "not-members", "platform tool refused");
            let who = strangers.join(", ");
            answerer.answer(id, self.tools.answer("not-members", &[("who", &who)], true));
            return Ok(());
        }
        let turn = self
            .groups
            .get(&session)
            .and_then(|group| group.turn(&line));
        if let Some(turn) = turn {
            let numbers = users.iter().map(i64::to_string).collect();
            self.mentions.0.insert(line, (turn, numbers));
        }
        tracing::info!(target: TARGET, venue = %peer.venue, tool, count = users.len(), "platform tool done");
        let who = names.join(", ");
        answerer.answer(id, self.tools.answer("mentioned", &[("who", &who)], false));
        Ok(())
    }
}

#[cfg(test)]
mod tests;
