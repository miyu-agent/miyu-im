//! 平台工具（一）的那一头（施工 O-31，`onebot.md` 第一条「平台工具（一）」第 2、5、6 条）：读的一头把撤回、禁言、戳一戳的
//! `tool.call` 照推送交到这里。先把这个会话留着的推送收进投影（私聊的照「怎么走」第 9 条那条路收，记下最近一条的引用），认出
//! 叫她做的那条、谁叫的；交 `platform.rs` 定对谁做，认不出目标的当场答。做的另起一个任务（`chores`，同贴表情）：她给的撤回编号先
//! `get_msg` 核对在不在这个会话里，再经收进这个会话的那个机器人号那时的连接调动作，答核心。跟核心的那一头不等 NapCat（「施工时
//! 定的」第 178 条）。施工 O-48 起不挡人（谁叫的、禁谁都交给 QQ）。

use std::sync::Arc;
use std::time::Instant;

use miyu_client::ids::ExternalId;
use serde_json::Value;

use super::fields::{NAME, clean};
use super::platform::{Caller, Order, Origin, Plan, Quote, Scene, Why, duration, plan, recallable};
use super::{Peer, Route};
use crate::TARGET;
use crate::core::{Answerer, Gone};
use crate::listen::bots::Bots;
use crate::media::{Missed, message};
use crate::onebot::{CallError, To, delete_msg, group_ban, person, poke, said};
use crate::rules::{AVATAR, FETCH_MEDIA, MEMBERS, Tools};

/// 要调的平台动作（号已经解成整数）。
#[derive(Debug, Clone)]
enum Act {
    /// 撤这一条（平台编号原样）；`check` 的先 `get_msg` 核对在不在这个会话里（施工 O-48）。
    Recall {
        /// 平台编号。
        msg: String,
        /// 先核对。
        check: bool,
    },
    /// 在这个群里禁言这个号几秒。
    Mute {
        /// 群号。
        group: i64,
        /// 禁谁。
        user: i64,
        /// 几秒，0 是解禁。
        seconds: u64,
    },
    /// 戳这个号。
    Poke(i64),
}

/// 交给任务办的一次调用。
struct Job {
    /// `tool.call` 的编号：照原样答。
    id: Value,
    /// 工具名：记运行日志。
    tool: String,
    /// 收进这个会话的机器人号、发到哪、场所。
    peer: Peer,
    /// 调什么。
    act: Act,
    /// 结果里写的人：名字，没有的写号（「施工时定的」第 185 条）。
    who: String,
    /// 机器人号的连接。
    bots: Arc<Bots>,
    /// 答的话。
    tools: Arc<Tools>,
    /// 往核心写回应的一头。
    answerer: Answerer,
}

impl Route {
    /// 核心转来的一次平台工具的调用 `pushed`（`tool.call`，「平台工具（一）」第 2 到 5 条；`fetch_media` 交给 `fetching`，施工
    /// O-33）。桥不认识这个会话的答
    /// `unreachable`；私聊里调到禁言的（`venues` 不给，照说不会）照不认识的工具答。
    ///
    /// # Errors
    ///
    /// 收这个会话留着的推送时写不出去、核心断开（同 [`Route::catch_up`]）。
    pub(super) async fn tool_called(&mut self, pushed: &Value) -> Result<(), Gone> {
        let params = &pushed["params"];
        let (id, tool) = (&pushed["id"], params["tool"].as_str().unwrap_or_default());
        if tool == FETCH_MEDIA {
            return self.fetch_called(pushed).await;
        }
        // 头像、群成员（施工 O-54，`lookup`）。
        if tool == AVATAR || tool == MEMBERS {
            return self.looked_up(pushed).await;
        }
        // 调用所在那条线（`line`）是支线的，照主线找投影、发到哪（施工 O-36）。
        let line = params["session"].as_str().unwrap_or_default().to_string();
        let session = self.main_of(&line);
        let answerer = self.core.answerer();
        self.gather(&session).await?;
        let Some(peer) = self.peers.get(&session).cloned() else {
            tracing::warn!(target: TARGET, session, tool, why = "unreachable", "platform tool failed");
            answerer.answer(id, self.tools.answer("unreachable", &[], true));
            return Ok(());
        };
        let caller = Caller::read(params);
        let decided = Order::read(tool, &params["args"])
            .and_then(|order| self.decide((&session, &line), &peer, order, &caller));
        let Some(decided) = decided else {
            answerer.answer(id, self.tools.call(tool));
            return Ok(());
        };
        let act = match self.act(&peer, decided) {
            Ok(act) => act,
            Err(why) => {
                tracing::info!(target: TARGET, venue = %peer.venue, tool, why = why.name(), "platform tool refused");
                answerer.answer(id, self.tools.answer(why.name(), &[], true));
                return Ok(());
            }
        };
        let who = match &act {
            Act::Recall { .. } => String::new(),
            Act::Mute { user, .. } | Act::Poke(user) => self.who_number(&peer, *user),
        };
        let job = Job {
            id: id.clone(),
            tool: tool.to_string(),
            peer,
            act,
            who,
            bots: Arc::clone(&self.bots),
            tools: Arc::clone(&self.tools),
            answerer,
        };
        self.chores.spawn(run(job));
        Ok(())
    }

    /// 会话 `session`（发到 `peer`）里 `caller` 叫她做的 `order` 对谁做（「平台工具（一）」第 3、4 条）：群里叫她做的那条照投影里
    /// 调用所在那条线 `line`（主线或者支线，施工 O-36），私聊照最近一条的引用。私聊里的禁言是空的。
    fn decide(
        &self,
        (session, line): (&str, &str),
        peer: &Peer,
        order: Order,
        caller: &Caller,
    ) -> Option<Plan> {
        let me = person(peer.bot).ok()?;
        match peer.to {
            To::Group(_) => {
                let scene = Scene {
                    peer: None,
                    me: &me,
                };
                let origin = self.groups.get(session).and_then(|one| one.origin(line));
                Some(plan(order, caller, origin.as_ref(), &scene))
            }
            To::Private(_) if matches!(order, Order::Mute { .. }) => None,
            To::Private(user) => {
                let other = person(user).ok()?;
                let quote = self.quotes.get(session).cloned().flatten();
                let origin = Origin {
                    sender: other.clone(),
                    quote: quote.map(|msg| Quote { msg, sender: None }),
                    mentions: Vec::new(),
                };
                let scene = Scene {
                    peer: Some(&other),
                    me: &me,
                };
                Some(plan(order, caller, Some(&origin), &scene))
            }
        }
    }

    /// 把会话 `session` 留着的推送收进来：群的进投影（同「群里怎么叫她」第 3 条），私聊的照「怎么走」第 9 条那条路（记下最近一条
    /// 的引用）。
    pub(super) async fn gather(&mut self, session: &str) -> Result<(), Gone> {
        if self.groups.contains_key(session) {
            return self.catch_up(session).await;
        }
        for pushed in self.core.take_events(session) {
            self.say_privately(session, &pushed["params"]["event"])
                .await?;
        }
        Ok(())
    }

    /// 定下来的 `plan` 换成要调的动作：号解成整数。认不出目标的交回为什么。
    fn act(&self, peer: &Peer, plan: Plan) -> Result<Act, Why> {
        let number = |user: &ExternalId| {
            miyu_chat::parse_person(user)
                .and_then(|(_, number)| number.parse::<i64>().ok())
                .ok_or(Why::NoTarget)
        };
        match plan {
            Plan::Refuse(why) => Err(why),
            Plan::Recall { msg, check } => Ok(Act::Recall { msg, check }),
            Plan::Poke(user) => Ok(Act::Poke(number(&user)?)),
            Plan::Mute { user, seconds } => {
                let id = number(&user)?;
                // 私聊没有禁言（`venues` 只给群，调的一方挡在前面）。
                let To::Group(group) = peer.to else {
                    return Err(Why::NoTarget);
                };
                Ok(Act::Mute {
                    group,
                    user: id,
                    seconds,
                })
            }
        }
    }

    /// 结果里写的号是 `user` 的人：群里照群成员缓存里的名字（洗过），没有的、私聊里的写号（「施工时定的」第 185 条）。
    fn who_number(&self, peer: &Peer, user: i64) -> String {
        let name = match peer.to {
            To::Group(group) => self
                .members
                .name(group, user, Instant::now())
                .and_then(|name| clean(name, NAME)),
            To::Private(_) => None,
        };
        name.unwrap_or_else(|| user.to_string())
    }
}

/// 没做成的：答的那一句的名字（`failed`、`unreachable`、`unanswered`；她给的撤回编号不在这个会话里的 `not-found`），`failed` 的
/// 带 NapCat 说的原因。
#[derive(Debug, PartialEq, Eq)]
struct Undone(&'static str, Option<String>);

/// 办一次调用：她给的撤回编号先核对，再调动作，答核心、记运行日志（「平台工具（一）」第 5、6 条）。
async fn run(job: Job) {
    let venue = &job.peer.venue;
    let who = job.who.as_str();
    let seconds;
    let outcome = act(&job).await;
    let (name, fields, error): (&str, Vec<(&str, &str)>, bool) = match &outcome {
        Ok(()) => {
            tracing::info!(target: TARGET, venue = %venue, tool = job.tool, "platform tool done");
            match &job.act {
                Act::Recall { .. } => ("recalled", Vec::new(), false),
                Act::Mute { seconds: 0, .. } => ("unmuted", vec![("who", who)], false),
                Act::Mute { seconds: muted, .. } => {
                    seconds = duration(*muted);
                    ("muted", vec![("who", who), ("duration", &seconds)], false)
                }
                Act::Poke(_) => ("poked", vec![("who", who)], false),
            }
        }
        Err(Undone(why, detail)) => {
            tracing::warn!(target: TARGET, venue = %venue, tool = job.tool, why, "platform tool failed");
            let fields = detail.as_deref().map(|detail| ("detail", detail));
            (why, fields.into_iter().collect(), true)
        }
    };
    job.answerer
        .answer(&job.id, job.tools.answer(name, &fields, error));
}

/// 调 `job` 的动作，经那个机器人号这时的连接；她给的撤回编号先 `get_msg` 核对（施工 O-48）：NapCat 说没有、不在这个会话里的
/// 答 `not-found`。
async fn act(job: &Job) -> Result<(), Undone> {
    let Some(link) = job.bots.get(job.peer.bot) else {
        return Err(Undone("unreachable", None));
    };
    let (action, params) = match &job.act {
        Act::Recall { msg, check } => {
            if *check {
                let data = match message(&link, msg).await {
                    Ok(data) => data,
                    Err(Missed::Failed(_)) => return Err(Undone("not-found", None)),
                    Err(Missed::Unanswered) => return Err(Undone("unanswered", None)),
                    Err(_) => return Err(Undone("unreachable", None)),
                };
                if !recallable(&data, job.peer.to, job.peer.bot) {
                    return Err(Undone("not-found", None));
                }
            }
            delete_msg(msg.as_str())
        }
        Act::Mute {
            group,
            user,
            seconds,
        } => group_ban(*group, *user, *seconds),
        Act::Poke(user) => poke(job.peer.to, *user),
    };
    link.calls
        .call(&link.out, action, params)
        .await
        .map(|_| ())
        .map_err(undone)
}

/// NapCat 没成算成哪一句：回了失败的是 `failed`，原因照它说的（`onebot::said`：`message`，空的换 `wording`，再空的写 `retcode`，
/// 去掉首尾空白、截到 200 个字符）；等不到是 `unanswered`；没连着、断了是 `unreachable`。
fn undone(error: CallError) -> Undone {
    match error {
        CallError::Failed(reply) => Undone("failed", Some(said(&reply))),
        CallError::Timeout => Undone("unanswered", None),
        CallError::Closed => Undone("unreachable", None),
    }
}

#[cfg(test)]
mod tests;
