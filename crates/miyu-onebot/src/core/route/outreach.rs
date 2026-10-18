//! 主动发消息（施工 O-56，`onebot.md` 第一条「主动发消息」，2026-10-17 项目主人定：她的好友、她在的群她都能发，这是拟人的一种设计）：
//! 工具 `chat_send`（她自己调）和给别的包调的方法 `chat.send`（以她的名义发，同她自己说话）走同一条路。
//!
//! - 发到哪：`user`（QQ 号）、`group`（群号）写一个。发不发、发给谁由她（或者调的包）定，桥不挡；QQ 不让的照 NapCat 的原话回。
//! - 发什么：照那个场所的出站链过一遍（清理、去末尾句号；不引用、不 @，同私聊），拆好的一段一条，经收进叫她的那个会话的机器人号
//!   （别的包调的、找不到的照连着的号里最小的那一个）调 `send_group_msg`、`send_private_msg`，不走出站队列：一段段等 NapCat 回了
//!   再答。太长的照对方场所的长文办（施工 O-59 下，`long`）：转成图的在这个任务里画、一页一条（画不成的照拆好的几段），合并
//!   转发的发一条。
//! - 对方那个场所的会话找得到的（群；终端管理员、白名单成员的私聊），发成了以后经核心的 `session.note` 给它记一块 `reached`
//!   事实（模板 `facts/reached.txt`，给模型看的字，登记在 `docs/ledger.md`）：对方回了，她知道是自己先找的人。陌生人的私聊没有
//!   会话，不记。

use std::collections::BTreeMap;
use std::sync::Arc;

use miyu_chat::{LongText, Venue, VenueKind};
use miyu_client::ids::VenueId;
use miyu_client::protocol::Template;
use serde_json::{Value, json};

use super::Route;
use super::long::{WAIT, draw, nodes, wants};
use super::outbound::{pass, private_ctx};
use super::session::Place;
use crate::TARGET;
use crate::chat::{CallError, Lead, To};
use crate::core::methods::ours;
use crate::core::{Answerer, Caller, Gone, reason};
use crate::listen::bots::Bots;
use crate::onebot::{forward_to, message_to, person, picture_to, said, venue};
use crate::rules::Tools;

/// `reached` 事实的 `kind`。
const REACHED: &str = "reached";

/// 事实里她那句话的开头最多几个字符（同退信）。
const HEAD: usize = 30;

/// 一次最多发多少个字符：拆段以前。
const MOST: usize = 4000;

/// 读 `chat_send`、`chat.send` 的参数（纯逻辑）：发到哪（`user`、`group` 写正好一个，正整数，写成字的也认），发什么（去掉首尾
/// 空白不空、不超过 [`MOST`] 个字符）。写错的是空的。
pub(super) fn request(args: &Value) -> Option<(To, String)> {
    let number = |value: &Value| match value {
        Value::Null => Some(None),
        Value::Number(number) => number.as_i64().filter(|n| *n > 0).map(Some),
        Value::String(text) => text.trim().parse::<i64>().ok().filter(|n| *n > 0).map(Some),
        _ => None,
    };
    let to = match (number(&args["user"])?, number(&args["group"])?) {
        (Some(user), None) => To::Private(user),
        (None, Some(group)) => To::Group(group),
        _ => return None,
    };
    let text = args["text"].as_str()?.trim();
    let fits = !text.is_empty() && text.chars().count() <= MOST;
    fits.then(|| (to, text.to_string()))
}

/// 交给任务办的一次。
struct Job {
    /// 经哪个机器人号。
    bot: i64,
    /// 发到哪。
    to: To,
    /// 拆好的几段：拆成几条的照它发；转成图画不成的也照它（施工 O-59 下）。
    pieces: Vec<String>,
    /// 长文怎么发（施工 O-59 下，`long`）。
    long: Long,
    /// 对方的场所会话：发成了记 `reached`；没有的不记。
    session: Option<String>,
    /// 机器人号的连接。
    bots: Arc<Bots>,
    /// 记事实。
    caller: Caller,
    /// `reached` 的模板。
    template: Template,
    /// 答谁。
    answer: Answer,
}

/// 长文怎么发（施工 O-59 下）：照对方场所的参数。
enum Long {
    /// 不长、拆成几条的：照 `pieces` 一段一条。
    Split,
    /// 转成图：在发的任务里调渲染包画（同她的话，`long::draw`），场所的编号、全文。
    Image(VenueId, String),
    /// 合并转发：几段、节点上的名字。
    Forward(Vec<String>, String),
}

/// 发完了答谁：她调的工具（`tool.call`），别的包调的方法（`method.call`）。
enum Answer {
    /// 工具：编号、答的话。
    Tool(Value, Arc<Tools>, Answerer),
    /// 方法：编号。
    Method(Value, Answerer),
}

impl Route {
    /// 核心转来的一次 `chat_send`（`pushed` 是 `tool.call`）：写错的答 `no-target`，别的起一个任务发。
    ///
    /// # Errors
    ///
    /// 收这个会话留着的推送、找对方的会话时写不出去、核心断开。
    pub(super) async fn send_called(&mut self, pushed: &Value) -> Result<(), Gone> {
        let params = &pushed["params"];
        let (id, tool) = (
            pushed["id"].clone(),
            params["tool"].as_str().unwrap_or_default(),
        );
        let line = params["session"].as_str().unwrap_or_default().to_string();
        let session = self.main_of(&line);
        let answerer = self.core.answerer();
        self.gather(&session).await?;
        let Some((to, text)) = request(&params["args"]) else {
            tracing::info!(target: TARGET, session, tool, why = "no-target", "platform tool refused");
            answerer.answer(&id, self.tools.answer("no-target", &[], true));
            return Ok(());
        };
        let bot = self.peers.get(&session).map(|peer| peer.bot);
        let answer = Answer::Tool(id, Arc::clone(&self.tools), answerer);
        self.reach(bot, (to, text), answer).await
    }

    /// 别的包经 `package.call` 调的一次 `chat.send`（`pushed` 是 `method.call`）：写错的拒 `bad_params`，别的起一个任务发；
    /// `bot` 写了的照它。
    ///
    /// # Errors
    ///
    /// 找对方的会话时写不出去、核心断开。
    pub(super) async fn send_exported(&mut self, pushed: &Value) -> Result<(), Gone> {
        let id = pushed["id"].clone();
        let args = &pushed["params"]["params"];
        let answerer = self.core.answerer();
        let from = pushed["params"]["from"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let Some(request) = ours(args).then(|| request(args)).flatten() else {
            tracing::info!(target: TARGET, from, "export send refused");
            answerer.refuse(&id, "bad_params");
            return Ok(());
        };
        let bot = args["bot"].as_i64();
        self.reach(bot, request, Answer::Method(id, answerer)).await
    }

    /// 照对方的场所过出站链、找对方的会话，起一个任务发（见模块开头）。没有机器人号连着的答 `unreachable`、拒 `not_connected`。
    async fn reach(
        &mut self,
        bot: Option<i64>,
        (to, text): (To, String),
        answer: Answer,
    ) -> Result<(), Gone> {
        let bot = bot
            .filter(|bot| self.bots.get(*bot).is_some())
            .or_else(|| self.bots.first().map(|(bot, _)| bot));
        let made = match to {
            To::Group(group) => venue(VenueKind::Group, group),
            To::Private(user) => venue(VenueKind::Private, user),
        };
        let (Some(bot), Ok(target)) = (bot, made) else {
            finish(answer, Err(("unreachable", None)));
            return Ok(());
        };
        let applied = self.applied(&target);
        let ctx = private_ctx(Vec::new(), applied.params.outbound.clone());
        let passed = match pass(&text, &ctx, None, applied.params.split_chars) {
            Ok(passed) => passed,
            Err(_) => {
                finish(answer, Err(("no-target", None)));
                return Ok(());
            }
        };
        let long = match wants(&applied.params, &passed.text) {
            Some(LongText::Image) => Long::Image(target.id().clone(), passed.text.clone()),
            Some(LongText::Forward) => Long::Forward(
                nodes(&applied.params, &passed.text),
                self.nickname_at(to, bot),
            ),
            Some(LongText::Split) | None => Long::Split,
        };
        let pieces = passed.pieces;
        let session = self.target_session(&target, to, bot).await?;
        let job = Job {
            bot,
            to,
            pieces,
            long,
            session,
            bots: Arc::clone(&self.bots),
            caller: self.core.caller(),
            template: self.rules.reached_fact().clone(),
            answer,
        };
        self.chores.spawn(run(job));
        Ok(())
    }

    /// 对方的场所会话（`target`，发到 `to`，经机器人号 `bot`）：群的照群消息的办法找（没有的造）；私聊的只找终端管理员、白名单
    /// 成员的，陌生人的不接、是空的。
    async fn target_session(
        &mut self,
        target: &Venue,
        to: To,
        bot: i64,
    ) -> Result<Option<String>, Gone> {
        let applied = self.applied(target);
        let place = match to {
            To::Group(group) => Place::group(target, &applied, bot, group),
            To::Private(user) => {
                let Ok(external) = person(user) else {
                    return Ok(None);
                };
                let listed = self.whitelisted(&external).then_some(&applied);
                Place::private(target, &external, (bot, user), listed)
            }
        };
        if !self.found(&place).await? {
            return Ok(None);
        }
        Ok(self.venues.get(target.id().as_str()).cloned())
    }
}

/// 发、记事实、答：一段段等 NapCat；都成了的记 `reached`、答 `sent`；有一段没成的停下、照它答（前面发成了的不撤）。
async fn run(job: Job) {
    let outcome = send(&job).await;
    if outcome.is_ok()
        && let Some(session) = &job.session
    {
        note(&job, session).await;
    }
    finish(job.answer, outcome);
}

/// 没发成的：答的那一句（拒的原因）、NapCat 说的。
type Undone = (&'static str, Option<String>);

/// 一条条发，交回发了几条：照长文的办法拼好几个动作（转成图的先画，画不成的照拆好的几段）。
async fn send(job: &Job) -> Result<usize, Undone> {
    let Some(link) = job.bots.get(job.bot) else {
        return Err(("unreachable", None));
    };
    let words = || -> Vec<(&'static str, Value)> {
        job.pieces
            .iter()
            .map(|piece| message_to(job.to, piece, &Lead::default()))
            .collect()
    };
    let actions = match &job.long {
        Long::Split => words(),
        Long::Image(venue, text) => match draw(&job.caller, venue, text, WAIT).await {
            Some(drawn) => drawn
                .iter()
                .map(|page| picture_to(job.to, &page.file, &Lead::default()))
                .collect(),
            None => words(),
        },
        Long::Forward(nodes, nickname) => vec![forward_to(job.to, job.bot, nickname, nodes)],
    };
    for (action, params) in &actions {
        link.calls
            .call(&link.out, action, params.clone())
            .await
            .map_err(|error| match error {
                CallError::Failed(reply) => ("failed", Some(said(&reply))),
                CallError::Timeout => ("unanswered", None),
                CallError::Closed => ("unreachable", None),
            })?;
    }
    Ok(actions.len())
}

/// 给对方的会话记一块 `reached`：她那句话的开头。核心拒了的记一行，不再试。
async fn note(job: &Job, session: &str) {
    let text: String = job.pieces.join("\n").chars().take(HEAD).collect();
    let fields = BTreeMap::from([("text", text.as_str())]);
    let Ok(fact) = job.template.render(&fields) else {
        // 照说不会：起来时查过模板只要这一个字段（`rules/facts.rs`）。
        return;
    };
    let facts = json!([{"kind": REACHED, "text": fact}]);
    match job
        .caller
        .call("session.note", json!({"session": session, "facts": facts}))
        .await
    {
        Ok(reply) => {
            if let Some(reason) = reason(&reply) {
                tracing::warn!(target: TARGET, session, reason, "note refused");
            }
        }
        Err(_) => tracing::warn!(target: TARGET, session, "note not written, core gone"),
    }
}

/// 答：工具照答的话（`sent`，没成的照那一句、算出错），方法照 `{sent: <几段>}`、没成的拒那个原因（NapCat 说的不带）。
fn finish(answer: Answer, outcome: Result<usize, Undone>) {
    match (answer, outcome) {
        (Answer::Tool(id, tools, answerer), Ok(_)) => {
            tracing::info!(target: TARGET, tool = "chat_send", "platform tool done");
            answerer.answer(&id, tools.answer("sent", &[], false));
        }
        (Answer::Tool(id, tools, answerer), Err((why, detail))) => {
            tracing::warn!(target: TARGET, tool = "chat_send", why, "platform tool failed");
            let fields: Vec<(&str, &str)> = detail
                .as_deref()
                .map(|detail| ("detail", detail))
                .into_iter()
                .collect();
            answerer.answer(&id, tools.answer(why, &fields, true));
        }
        (Answer::Method(id, answerer), Ok(pieces)) => {
            tracing::info!(target: TARGET, pieces, "export send");
            answerer.answer(&id, json!({"sent": pieces}));
        }
        (Answer::Method(id, answerer), Err((why, _))) => {
            let why = match why {
                "unreachable" => "not_connected",
                other => other,
            };
            tracing::info!(target: TARGET, why, "export send failed");
            answerer.refuse(&id, why);
        }
    }
}

#[cfg(test)]
mod tests;
