//! 驱动交的工具的调用（施工 T-1，`crate::toolset`）：读的一头把它们的 `tool.call` 照推送交到这里。照会话填好场景（发到哪、机器人
//! 号、谁叫的、工作区、那个机器人号的连接；施工 T-2 多叫她做的那一条、她自己、私聊的对方、这个群缓存着的名字）交给驱动：当场答的写回去；要去平台上办的另起一个任务（`chores`，同取图）等它办完再
//! 答，跟核心的那一头不等。桥不认识这个会话的（本机的会话）照驱动的 `ToolSet::local` 办（施工 I-40：主动发消息），别的答 `unreachable`。
//!
//! 挂几个驱动的（施工 I-70 中上）：平台、配置、状态目录照这件工具是哪个驱动的那一套取；场所是别的平台的不调（答 `unreachable`，
//! 运行日志记 `another_platform`），本机的会话照那一套的平台发。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use miyu_client::ids::ExternalId;
use serde_json::Value;

use super::Route;
use super::elsewhere::{Aimed, Asked};
use super::outreach::Answer;
use super::session::Peer;
use crate::TARGET;
use crate::chat::{Id, To};
use crate::core::Answerer;
use crate::core::Gone;
use crate::rules::Toolset;
use crate::toolset::{Blobs, Call, Done, Effect, NAME, Origin, Planned, Quote, clean};

impl Route {
    /// 核心转来的一次驱动的工具的调用（`pushed` 是 `tool.call`），那一套是平台 `platform` 的驱动交的 `set`、答的话是 `answers`。
    ///
    /// # Errors
    ///
    /// 收这个会话留着的推送时写不出去、核心断开。
    pub(super) async fn driven_called(
        &mut self,
        pushed: &Value,
        toolset: Toolset,
    ) -> Result<(), Gone> {
        let params = &pushed["params"];
        let asked = Asked {
            id: pushed["id"].clone(),
            tool: params["tool"].as_str().unwrap_or_default().to_string(),
            line: params["session"].as_str().unwrap_or_default().to_string(),
            answerer: self.core.answerer(),
        };
        let session = self.main_of(&asked.line);
        self.gather(&session).await?;
        let Some(peer) = self.peers.get(&session).cloned() else {
            // 不是场所会话（施工 I-40）：本机的会话（`elsewhere.rs`）。
            return self.local_called(params, asked, toolset).await;
        };
        let (platform, set, answers) = &toolset;
        if peer.platform != *platform {
            // 别的平台的工具（施工 I-70 中上）：核心照名字的前缀只给那个平台的场所（IM-3），到了这里是对不上的，不叫驱动做。
            tracing::warn!(target: TARGET, session, tool = asked.tool, why = "another_platform", "platform tool failed");
            asked
                .answerer
                .answer(&asked.id, answers.say("unreachable", &[], true));
            return Ok(());
        }
        // 写了别的场所的（设计 13）：只有终端管理员直接叫她的那一轮去那个场所办（同 I-204 的认法），别人叫的拒。
        match super::elsewhere::aimed(set.as_ref(), &asked.tool, &params["args"]) {
            Aimed::Bad => {
                asked
                    .answerer
                    .answer(&asked.id, answers.say("chat-bad", &[], true));
                return Ok(());
            }
            Aimed::At(to) if to != peer.to => {
                let (admin, commanded) = self.standing(&session, &asked.line, &peer, params);
                if admin && commanded {
                    let bot = Some(peer.bot.clone());
                    return self
                        .elsewhere(params, asked, toolset, (Some(to), bot))
                        .await;
                }
                asked
                    .answerer
                    .answer(&asked.id, answers.say("only-here", &[], true));
                return Ok(());
            }
            _ => {}
        }
        self.venue_called(params, asked, (session, peer), toolset, None)
            .await
    }

    /// 这一轮是不是终端管理员叫的、是不是管理的人直接叫她开的：交回 `(admin, commanded)`。
    fn standing(&mut self, session: &str, line: &str, peer: &Peer, params: &Value) -> (bool, bool) {
        let by = by_of(params);
        // 这一轮是不是终端管理员叫的（施工 C-1 下）：私聊里核心认出了人（`by` 是 `person`），群里照投影里他说的话带的 `owner`。
        let admin = params["by"]["kind"] == "person"
            || by.as_ref().is_some_and(|by| {
                self.groups
                    .get(session)
                    .is_some_and(|group| group.is_admin(by))
            });
        // 这一轮是不是管理的人直接叫她开的（施工 I-204，删群文件这种收不回来的事照它）：群里照投影认（`Projection::commanded`：开
        // 这一轮的那一条冲她来，在回的每一条都是终端管理员、卡片上 `managers` 里的人发的）；私聊里就是终端管理员叫的。
        let commanded = match &peer.to {
            To::Private(_) => admin,
            To::Group(_) => {
                let rules = miyu_chat::Venue::parse(&peer.venue).map(|venue| self.applied(&venue));
                self.groups.get(session).is_some_and(|group| {
                    group.commanded(line, |who| {
                        rules
                            .as_ref()
                            .is_some_and(|rules| super::applied::role(rules, who) == "manager")
                    })
                })
            }
        };
        (admin, commanded)
    }

    /// 在场所 `peer`（会话 `session`）里办这一次调用 `asked`。`owner` 是终端里、或者终端管理员叫她去别的场所办的（设计 13）：照
    /// 终端管理员算，答的话前头写上它（是哪个场所；不找场所的是空的字）。
    pub(super) async fn venue_called(
        &mut self,
        params: &Value,
        asked: Asked,
        (session, peer): (String, Peer),
        (platform, set, answers): Toolset,
        owner: Option<String>,
    ) -> Result<(), Gone> {
        let Asked {
            id,
            tool,
            line,
            answerer,
        } = asked;
        let tool = tool.as_str();
        let by = owner.is_none().then(|| by_of(params)).flatten();
        let (caller, other, origin, names) = match peer.to.clone() {
            To::Private(user) => {
                // 私聊：叫她做的那条是最近一条人说的话（对方的），引用照记下的（「怎么走」第 9 条）。
                let other = peer.platform.person(&user).ok();
                let quote = self.quotes.get(&session).cloned().flatten();
                let origin = other
                    .clone()
                    .filter(|_| owner.is_none())
                    .map(|sender| Origin {
                        sender,
                        quote: quote.map(|msg| Quote { msg, sender: None }),
                        mentions: Vec::new(),
                    });
                (
                    owner.is_none().then_some(user),
                    other,
                    origin,
                    HashMap::new(),
                )
            }
            To::Group(group) => {
                // 群：投影里调用所在那条线（主线或者支线，施工 O-36）这一轮她回的那一条。终端里的没有。
                let origin = self
                    .groups
                    .get(&session)
                    .filter(|_| owner.is_none())
                    .and_then(|one| one.origin(&line));
                let names = self
                    .members
                    .names_in(peer.platform, &group, Instant::now())
                    .into_iter()
                    .filter_map(|(user, name)| Some((user, clean(&name, NAME)?)))
                    .collect();
                (by.as_ref().and_then(number), None, origin, names)
            }
        };
        // 这一轮这件工具以前调过几次（施工 T-3）：群照投影里调用所在那条线这一轮，私聊照推来的事件的回合；换了回合从头数。
        let turn = match self.groups.get(&session).filter(|_| owner.is_none()) {
            Some(group) => group.turn(&line),
            None => self.turns.get(&line).copied().flatten(),
        };
        let used = counted(
            self.uses
                .entry((line.clone(), tool.to_string()))
                .or_insert((turn, 0)),
            turn,
        );
        let (admin, commanded) = match owner.as_ref() {
            Some(_) => (true, true),
            None => self.standing(&session, &line, &peer, params),
        };
        let call = Call {
            tool: tool.to_string(),
            args: params["args"].clone(),
            venue: peer.venue.as_str().to_string(),
            to: peer.to.clone(),
            bot: peer.bot.clone(),
            caller,
            by,
            origin,
            me: peer.platform.person(&peer.bot).ok(),
            other,
            platform: peer.platform,
            names,
            cwd: params["cwd"].as_str().unwrap_or_default().to_string(),
            line: line.clone(),
            used,
            blobs: Blobs(self.core.caller()),
            link: self.platforms.link(peer.platform, &peer.bot),
            answers,
            plugins: self
                .hooks
                .active(platform, Some(&peer.bot), peer.to.clone()),
            config: self.hooks.driver_config(platform),
            store: crate::status_file::store_in(&self.store, platform),
            admin,
            commanded,
            open: open(params),
            history: self.records.history(),
            usage: crate::toolset::Usage {
                path: self.store.join(crate::spent::FILE),
                caller: self.core.caller(),
            },
        };
        let (venue, tool) = (call.venue.clone(), call.tool.clone());
        let answers = Arc::clone(&call.answers);
        match set.plan(call) {
            Planned::Now(answer) => {
                let answer = labelled(answer, owner.as_deref());
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
                        label: owner,
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
                let names: HashMap<Id, String> = match &peer.to {
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
                            let name = Id::new(number).and_then(|user| names.get(&user).cloned());
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
                answerer.answer(&id, labelled(answer, owner.as_deref()));
            }
            // 定下一次看一眼（施工 I-89 下）：删掉等着的定时、照它建新的。
            Planned::Host(Effect::Glance { minutes }) => {
                self.glance_after(peer.venue.as_str(), minutes).await?;
                let minutes = minutes.to_string();
                answerer.answer(
                    &id,
                    answers.say("glance-set", &[("minutes", &minutes)], false),
                );
            }
            // 传文件（施工 I-130）：另起任务交给平台照路径传，当场答在传了；没传成的照 `show` 的文件记 `unshown`。
            Planned::Host(Effect::Upload { path, name, size }) => {
                let said = answers.say("uploading", &[("name", &name), ("size", &size)], false);
                self.upload((&session, &line), &peer, (path, name));
                logged(&venue, &tool, &said);
                answerer.answer(&id, said);
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
            label,
        } = finished;
        match (done.effect, turn) {
            (Some(Effect::Mention(users)), Some(turn)) => self.mention(line, turn, &users),
            (Some(Effect::Quote(msg)), Some(turn)) => self.quote(line, turn, msg),
            _ => {}
        }
        let answer = labelled(done.answer, label.as_deref());
        logged(&venue, &tool, &answer);
        answerer.answer(&id, answer);
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
    /// 终端里、去别的场所办的（设计 13）：答的话前头写上的那一句（是哪个场所）；别的是空的。
    label: Option<String>,
}

/// 答的话 `answer` 第一段前头写上 `label`（设计 13：终端里看不到「这个场所」，结果写明是哪个）；没有的、空的原样。
fn labelled(mut answer: Value, label: Option<&str>) -> Value {
    if let Some(label) = label.filter(|label| !label.is_empty())
        && let Some(text) = answer["blocks"][0]["text"].as_str()
    {
        answer["blocks"][0]["text"] = Value::from(format!("{label}\n{text}"));
    }
    answer
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
fn number(id: &ExternalId) -> Option<Id> {
    Id::new(id.as_str().rsplit_once(':')?.1)
}

/// 这次调用放开了没有（施工 LV-4，核心 TC-1 的 `tool.call`：`owner` 是真、`permission.level` 是 `full`、`read_only` 不是真）：
/// 纯逻辑，少了哪一格都算没放开。
pub(super) fn open(params: &Value) -> bool {
    let permission = &params["permission"];
    params["owner"] == true && permission["level"] == "full" && permission["read_only"] != true
}

#[cfg(test)]
mod tests;
