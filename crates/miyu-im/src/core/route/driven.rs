//! 驱动交的工具的调用（施工 T-1，`crate::toolset`）：读的一头把它们的 `tool.call` 照推送交到这里。照会话填好场景（发到哪、机器人
//! 号、谁叫的、工作区、那个机器人号的连接；施工 T-2 多叫她做的那一条、她自己、私聊的对方、这个群缓存着的名字）交给驱动：当场答的写回去；要去平台上办的另起一个任务（`chores`，同取图）等它办完再
//! 答，跟核心的那一头不等。桥不认识这个会话的答 `unreachable`。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use miyu_client::ids::ExternalId;
use serde_json::Value;

use super::Route;
use super::outreach::Answer;
use crate::TARGET;
use crate::chat::To;
use crate::core::Answerer;
use crate::core::Gone;
use crate::toolset::{
    Answers, Blobs, Call, Done, Effect, NAME, Origin, Planned, Quote, ToolSet, clean,
};

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
            answerer.answer(&id, answers.say("unreachable", &[], true));
            return Ok(());
        };
        let by = by_of(params);
        let (caller, other, origin, names) = match peer.to {
            To::Private(user) => {
                // 私聊：叫她做的那条是最近一条人说的话（对方的），引用照记下的（「怎么走」第 9 条）。
                let other = self.platform.person(user).ok();
                let quote = self.quotes.get(&session).cloned().flatten();
                let origin = other.clone().map(|sender| Origin {
                    sender,
                    quote: quote.map(|msg| Quote { msg, sender: None }),
                    mentions: Vec::new(),
                });
                (Some(user), other, origin, HashMap::new())
            }
            To::Group(group) => {
                // 群：投影里调用所在那条线（主线或者支线，施工 O-36）这一轮她回的那一条。
                let origin = self.groups.get(&session).and_then(|one| one.origin(&line));
                let names = self
                    .members
                    .names_in(group, Instant::now())
                    .into_iter()
                    .filter_map(|(user, name)| Some((user, clean(&name, NAME)?)))
                    .collect();
                (by.as_ref().and_then(number), None, origin, names)
            }
        };
        // 这一轮这件工具以前调过几次（施工 T-3）：群照投影里调用所在那条线这一轮，私聊照推来的事件的回合；换了回合从头数。
        let turn = match self.groups.get(&session) {
            Some(group) => group.turn(&line),
            None => self.turns.get(&line).copied().flatten(),
        };
        let used = counted(
            self.uses
                .entry((line.clone(), tool.to_string()))
                .or_insert((turn, 0)),
            turn,
        );
        let call = Call {
            tool: tool.to_string(),
            args: params["args"].clone(),
            venue: peer.venue.as_str().to_string(),
            to: peer.to,
            bot: peer.bot,
            caller,
            owner: params["by"]["kind"] == "person" || params["by"]["account"].is_string(),
            by,
            origin,
            me: self.platform.person(peer.bot).ok(),
            other,
            platform: self.platform,
            names,
            cwd: params["cwd"].as_str().unwrap_or_default().to_string(),
            line: line.clone(),
            used,
            blobs: Blobs(self.core.caller()),
            link: self.bots.get(peer.bot),
            answers,
            plugins: self.hooks.active(peer.to),
            config: self.hooks.driver_config(),
            store: self.store.clone(),
        };
        let (venue, tool) = (call.venue.clone(), call.tool.clone());
        let answers = Arc::clone(&call.answers);
        match set.plan(call) {
            Planned::Now(answer) => {
                logged(&venue, &tool, &answer);
                answerer.answer(&id, answer);
            }
            Planned::Later(pending) => {
                self.jobs.spawn(async move {
                    let done = pending.await;
                    Finished {
                        id,
                        answerer,
                        venue,
                        tool,
                        line,
                        turn,
                        done,
                    }
                });
            }
            // 交给 im 当场做（施工 T-4）：主动发消息照对方场所的出站链发，发完照这一套的答的话答。
            Planned::Host(Effect::Send { to, text }) => {
                let answer = Answer::Tool(id, answers, answerer);
                return self.reach(Some(peer.bot), (to, text), answer).await;
            }
            // @ 谁要带着答的话交回（`Planned::Later` 的 `Done`）：当场交的是驱动写错了，照够不着答。
            Planned::Host(Effect::Mention(_)) => {
                answerer.answer(&id, answers.say("unreachable", &[], true));
            }
        }
        Ok(())
    }

    /// 驱动交回的任务办完了（施工 T-4）：先做它交回的那件事（@ 谁记在那条线、那一轮上），再答、记一行运行日志。
    pub(super) fn finished(&mut self, finished: Finished) {
        let Finished {
            id,
            answerer,
            venue,
            tool,
            line,
            turn,
            done,
        } = finished;
        if let (Some(Effect::Mention(users)), Some(turn)) = (done.effect, turn) {
            self.mention(line, turn, &users);
        }
        logged(&venue, &tool, &done.answer);
        answerer.answer(&id, done.answer);
    }
}

/// 驱动交回的一个任务办完了的（`Route::finished`）。
pub(crate) struct Finished {
    id: Value,
    answerer: Answerer,
    venue: String,
    tool: String,
    line: String,
    turn: Option<u64>,
    done: Done,
}

/// 记一行运行日志：出错的 `WARN platform tool failed`，成了的 `INFO platform tool done`；`said` 是答她的那句的开头（施工 I-16：
/// 首页活动流照它写做了什么）。
fn logged(venue: &str, tool: &str, answer: &Value) {
    let text: String = answer["blocks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|block| block["text"].as_str())
        .collect();
    let said = crate::recent::Note::excerpt(&text);
    if answer["error"] == true {
        tracing::warn!(target: TARGET, venue, tool, said = %said, "platform tool failed");
    } else {
        tracing::info!(target: TARGET, venue, tool, said = %said, "platform tool done");
    }
}

/// 谁叫的（`tool.call` 的 `by`，`providers.md`「是谁要的」）：外部身份的照 `by.id`；私聊里核心照对应表认出的终端管理员记成本人，照
/// 它经的平台身份 `by.via`。没有触发的回合、本机的人、别的种类是空的。
pub(super) fn by_of(params: &Value) -> Option<ExternalId> {
    let by = &params["by"];
    let id = match by["kind"].as_str() {
        Some("external") => by["id"].as_str(),
        Some("person") => by["via"].as_str(),
        _ => None,
    }?;
    ExternalId::parse(id).ok()
}

/// 记着的（那一轮的回合、调了几次）`kept` 照这一轮 `turn` 数这一次（纯逻辑）：换了一轮的从头数；交回这一次以前调过几次。
pub(super) fn counted(kept: &mut (Option<u64>, usize), turn: Option<u64>) -> usize {
    if kept.0 != turn {
        *kept = (turn, 0);
    }
    kept.1 += 1;
    kept.1 - 1
}

/// 平台上的人 `id` 的号。
fn number(id: &ExternalId) -> Option<i64> {
    id.as_str().rsplit_once(':')?.1.parse().ok()
}

#[cfg(test)]
mod tests;
