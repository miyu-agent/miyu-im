//! 主动发消息（施工 O-56，`onebot.md` 第一条「主动发消息」，2026-10-17 项目主人定：她的好友、她在的群她都能发，这是拟人的一种设计）：
//! 工具 `chat_send`（她自己调）和给别的包调的方法 `chat.send`（以她的名义发，同她自己说话）走同一条路。
//!
//! - 发到哪：`user`（QQ 号）、`group`（群号）写一个。发不发、发给谁由她（或者调的包）定，桥不挡；QQ 不让的照 NapCat 的原话回。
//! - 发什么：照那个场所的出站链过一遍（清理、去末尾句号；不引用、不 @，同私聊），拆好的一段一条，经收进叫她的那个会话的机器人号
//!   （别的包调的、找不到的照同一个平台连着的号里最小的那一个，施工 I-70 上：不跨平台）调 `send_group_msg`、`send_private_msg`，不走出站队列：一段段等 NapCat 回了
//!   再答。太长的照对方场所的长文办（施工 O-59 下，`long`）：转成图的在这个任务里画、一页一条（画不成的照拆好的几段），合并
//!   转发的发一条。
//! - 对方那个场所的会话找得到的（群；终端管理员、白名单成员的私聊），发成了以后经核心的 `session.note` 给它记一块 `reached`
//!   事实（模板 `facts/reached.txt`，给模型看的字，登记在 `docs/ledger.md`）：对方回了，她知道是自己先找的人。陌生人的私聊没有
//!   会话，不记。

use std::collections::BTreeMap;
use std::sync::Arc;

use miyu_chat::{LongText, Venue, VenueKind, split};
use miyu_client::ids::VenueId;
use miyu_client::protocol::Template;
use serde_json::{Value, json};

use super::Route;
use super::long::{WAIT, draw, nodes};
use super::outbound::{pass, private_ctx};
use super::session::Place;
use crate::TARGET;
use crate::chat::Outgoing;
use crate::chat::{Bots, Id, Platform};
use crate::chat::{CallError, Lead, To};
use crate::core::{Answerer, Caller, Gone, reason};
use crate::toolset::{Answers, send_request as request};

/// `reached` 事实的 `kind`。
const REACHED: &str = "reached";

/// 事实里她那句话的开头最多几个字符（同退信）。
const HEAD: usize = 30;

/// 交给任务办的一次。
struct Job {
    /// 经哪个机器人号。
    bot: Id,
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
pub(super) enum Answer {
    /// 工具（施工 T-4 起是驱动交的 `chat_send`）：编号、答的话。
    Tool(Value, Arc<Answers>, Answerer),
    /// 方法：编号。
    Method(Value, Answerer),
}

impl Route {
    /// 别的包经 `package.call` 调的一次 `chat.send`（`pushed` 是 `method.call`）：写错的拒 `bad_params`，别的起一个任务发；
    /// `bot` 写了的照它。平台照 `platform`（施工 I-70 上，`Platforms::ours`）：写了别的平台的、挂了几个平台又没写的拒 `bad_params`。
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
        let platform = match self.platforms.ours(args) {
            Ok(platform) => platform,
            Err(why) => {
                tracing::info!(target: TARGET, from, why = why.name(), "export send refused");
                answerer.refuse(&id, "bad_params");
                return Ok(());
            }
        };
        let Some(request) = request(args) else {
            tracing::info!(target: TARGET, from, "export send refused");
            answerer.refuse(&id, "bad_params");
            return Ok(());
        };
        let bot = serde_json::from_value::<Id>(args["bot"].clone()).ok();
        self.reach((platform, bot), request, Answer::Method(id, answerer))
            .await
    }

    /// 本机的会话里主动发消息经哪个号（施工 I-81 下下，设计 07 第 7.7 节第 5 条）：参数 `given` 写了、连着的照它；没写的，这个平台
    /// 连着的号不止一个的不猜。交回号（空的照 [`Route::reach`] 的老办法挑）；不知道经哪个的交回连着的号，照号排、逗号隔开。
    pub(super) fn local_sender(
        &self,
        platform: Platform,
        given: &Value,
    ) -> Result<Option<Id>, String> {
        let connected: Vec<Id> = self
            .platforms
            .bots(platform)
            .held()
            .into_iter()
            .map(|(bot, ..)| bot)
            .collect();
        let listed = || {
            connected
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        };
        match serde_json::from_value::<Id>(given.clone()).ok() {
            Some(bot) if connected.contains(&bot) => Ok(Some(bot)),
            Some(_) => Err(listed()),
            None if connected.len() > 1 => Err(listed()),
            None => Ok(None),
        }
    }

    /// 照对方的场所过出站链、找对方的会话，起一个任务发（见模块开头）。经平台 `platform` 上的号 `bot`，没写的、没连着的退回这个
    /// 平台连着的号里最小的那一个（不跨平台，施工 I-70 上）；这个平台没有号连着的答 `unreachable`、拒 `not_connected`。
    pub(super) async fn reach(
        &mut self,
        (platform, bot): (Platform, Option<Id>),
        (to, text): (To, String),
        answer: Answer,
    ) -> Result<(), Gone> {
        let bot = self.platforms.sender(platform, bot);
        let made = bot.as_ref().and_then(|bot| {
            match &to {
                To::Group(group) => platform.venue(bot, VenueKind::Group, group),
                To::Private(user) => platform.venue(bot, VenueKind::Private, user),
            }
            .ok()
        });
        let (Some(bot), Some(target)) = (bot, made) else {
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
        let long = match self.long_text(platform, &applied.params, &passed.text) {
            Some(LongText::Image) => Long::Image(target.id().clone(), passed.text.clone()),
            Some(LongText::Forward) => Long::Forward(
                nodes(&applied.params, &passed.text),
                self.nickname_at(platform, to.clone(), bot.clone()),
            ),
            Some(LongText::Split) | None => Long::Split,
        };
        // 发到别处的照旧按长度拆、一口气发（施工 I-26）：她调工具等着发完，照真人停会把这一步拖长几十秒。
        let pieces = match passed.pauses.is_empty() {
            true => passed.pieces,
            false => split(&passed.text, applied.params.split_chars),
        };
        let session = self
            .target_session((platform, &target), to.clone(), bot.clone())
            .await?;
        let job = Job {
            bot,
            to,
            pieces,
            long,
            session,
            bots: self.platforms.bots(platform),
            caller: self.core.caller(),
            template: self.rules.reached_fact().clone(),
            answer,
        };
        self.chores.spawn(run(job));
        Ok(())
    }

    /// 对方的场所会话（平台 `platform` 上的 `target`，发到 `to`，经机器人号 `bot`）：群的照群消息的办法找（没有的造）；私聊的只找终端管理员、白名单
    /// 成员的，陌生人的不接、是空的。
    async fn target_session(
        &mut self,
        (platform, target): (Platform, &Venue),
        to: To,
        bot: Id,
    ) -> Result<Option<String>, Gone> {
        let applied = self.applied(target);
        let place = match to {
            To::Group(group) => Place::group((platform, target), &applied, bot, group),
            To::Private(user) => {
                let Ok(external) = platform.person(&user) else {
                    return Ok(None);
                };
                let listed = self.whitelisted(&external).then_some(&applied);
                Place::private((platform, target), &external, (bot, user), listed)
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
    let Some(link) = job.bots.get(&job.bot) else {
        return Err(("unreachable", None));
    };
    let lead = Lead::default();
    let drawn = match &job.long {
        Long::Image(venue, text) => draw(&job.caller, venue, text, WAIT).await,
        Long::Split | Long::Forward(..) => None,
    };
    let outgoing: Vec<Outgoing<'_>> = match (&job.long, &drawn) {
        (Long::Forward(nodes, nickname), _) => vec![Outgoing::Forward {
            bot: &job.bot,
            nickname,
            nodes,
        }],
        (Long::Image(..), Some(drawn)) => drawn
            .iter()
            .map(|page| Outgoing::Picture {
                file: &page.file,
                lead: &lead,
            })
            .collect(),
        (Long::Split | Long::Image(..), _) => job
            .pieces
            .iter()
            .map(|piece| Outgoing::Text {
                text: piece,
                lead: &lead,
            })
            .collect(),
    };
    for one in &outgoing {
        link.send(job.to.clone(), *one)
            .await
            .map_err(|error| match error {
                CallError::Failed { said, .. } => ("failed", Some(said)),
                CallError::Timeout => ("unanswered", None),
                CallError::Closed => ("unreachable", None),
                CallError::Unsupported => ("failed", Some("unsupported".to_string())),
            })?;
    }
    Ok(outgoing.len())
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
            answerer.answer(&id, tools.say("sent", &[], false));
        }
        (Answer::Tool(id, tools, answerer), Err((why, detail))) => {
            tracing::warn!(target: TARGET, tool = "chat_send", why, "platform tool failed");
            let fields: Vec<(&str, &str)> = detail
                .as_deref()
                .map(|detail| ("detail", detail))
                .into_iter()
                .collect();
            answerer.answer(&id, tools.say(why, &fields, true));
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
