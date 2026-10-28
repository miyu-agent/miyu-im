//! 驱动交的工具的调用（施工 T-1，`crate::toolset`）：读的一头把它们的 `tool.call` 照推送交到这里。照会话填好场景（发到哪、机器人
//! 号、谁叫的、工作区、那个机器人号的连接；施工 T-2 多叫她做的那一条、她自己、私聊的对方、这个群缓存着的名字）交给驱动：当场答的写回去；要去平台上办的另起一个任务（`chores`，同取图）等它办完再
//! 答，跟核心的那一头不等。桥不认识这个会话的（本机的会话）照驱动的 `ToolSet::local` 办（施工 I-40：主动发消息），别的答 `unreachable`。

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
            // 不是场所会话（施工 I-40）：本机的会话里登记了 `local` 的那几件照驱动说的办（主动发消息经连着的号里最小的那一个）。
            // 没有场所，平台照参数认（施工 I-70 上，`Platforms::ours`：只挂了一个平台的算它）。
            match set.local(tool, &params["args"], &answers) {
                Some(Planned::Host(Effect::Send { to, text })) => {
                    match self.platforms.ours(&params["args"]) {
                        Ok(platform) => {
                            let answer = Answer::Tool(id, answers, answerer);
                            return self.reach((platform, None), (to, text), answer).await;
                        }
                        Err(why) => {
                            tracing::warn!(target: TARGET, session, tool, why = why.name(), "platform tool failed");
                            answerer.answer(&id, answers.say("unreachable", &[], true));
                        }
                    }
                }
                Some(Planned::Now(answer)) => answerer.answer(&id, answer),
                _ => {
                    tracing::warn!(target: TARGET, session, tool, why = "unreachable", "platform tool failed");
                    answerer.answer(&id, answers.say("unreachable", &[], true));
                }
            }
            return Ok(());
        };
        let by = by_of(params);
        let (caller, other, origin, names) = match peer.to {
            To::Private(user) => {
                // 私聊：叫她做的那条是最近一条人说的话（对方的），引用照记下的（「怎么走」第 9 条）。
                let other = peer.platform.person(user).ok();
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
                    .names_in(peer.platform, group, Instant::now())
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
        // 这一轮是不是终端管理员叫的（施工 C-1 下）：私聊里核心认出了人（`by` 是 `person`），群里照投影里他说的话带的 `account`。
        let admin = params["by"]["kind"] == "person"
            || by.as_ref().is_some_and(|by| {
                self.groups
                    .get(&session)
                    .is_some_and(|group| group.is_admin(by))
            });
        let call = Call {
            tool: tool.to_string(),
            args: params["args"].clone(),
            venue: peer.venue.as_str().to_string(),
            to: peer.to,
            bot: peer.bot,
            caller,
            by,
            origin,
            me: peer.platform.person(peer.bot).ok(),
            other,
            platform: peer.platform,
            names,
            cwd: params["cwd"].as_str().unwrap_or_default().to_string(),
            line: line.clone(),
            used,
            blobs: Blobs(self.core.caller()),
            link: self.platforms.link(peer.platform, peer.bot),
            answers,
            plugins: self.hooks.active(peer.to),
            config: self.hooks.driver_config(),
            store: self.store.clone(),
            admin,
            history: self.records.history(),
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
                return self
                    .reach((peer.platform, Some(peer.bot)), (to, text), answer)
                    .await;
            }
            // 发言排行（施工 I-20）：照 im 自己按人数的条数排，名字照群成员的缓存，没有的写号。
            Planned::Host(Effect::Ranking { days, limit }) => {
                let counted = self.counters.days(&crate::counters::last_days(days));
                let names: HashMap<i64, String> = match peer.to {
                    To::Group(group) => self
                        .members
                        .names_in(peer.platform, group, Instant::now())
                        .into_iter()
                        .collect(),
                    To::Private(_) => HashMap::new(),
                };
                let ranked = crate::counters::ranking(&counted, peer.venue.as_str());
                let days_said = days.to_string();
                let answer = if ranked.is_empty() {
                    answers.say("no-activity", &[("days", &days_said)], false)
                } else {
                    let list: Vec<String> = ranked
                        .iter()
                        .take(limit)
                        .enumerate()
                        .map(|(place, (person, count))| {
                            let number = person.rsplit(':').next().unwrap_or(person);
                            let name = number
                                .parse::<i64>()
                                .ok()
                                .and_then(|user| names.get(&user).cloned());
                            match name {
                                Some(name) => format!("{}. {name} ({number}): {count}", place + 1),
                                None => format!("{}. {number}: {count}", place + 1),
                            }
                        })
                        .collect();
                    let shown = list.len().to_string();
                    answers.say(
                        "activity",
                        &[
                            ("count", &shown),
                            ("days", &days_said),
                            ("list", &list.join("; ")),
                        ],
                        false,
                    )
                };
                answerer.answer(&id, answer);
            }
            // @ 谁、引用哪条要带着答的话交回（`Planned::Later` 的 `Done`）：当场交的是驱动写错了，照够不着答。
            Planned::Host(Effect::Mention(_) | Effect::Quote(_)) => {
                answerer.answer(&id, answers.say("unreachable", &[], true));
            }
        }
        Ok(())
    }

    /// 驱动交回的任务办完了（施工 T-4）：先做它交回的那件事（@ 谁、引用哪条记在那条线、那一轮上），再答、记一行运行日志。
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
        match (done.effect, turn) {
            (Some(Effect::Mention(users)), Some(turn)) => self.mention(line, turn, &users),
            (Some(Effect::Quote(msg)), Some(turn)) => self.quote(line, turn, msg),
            _ => {}
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
