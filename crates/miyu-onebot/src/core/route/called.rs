//! 叫她（施工 O-23，`onebot.md` 第一条「群里怎么叫她」第 3 到 8、10、11、14 条）：一条群消息记下以后，先把这个会话留着没办的
//! 推送收进投影（核心先推、后回应：核心记下的这一条、刚开的回合都在里面），再照场所规则、投影、桥内存里在判的、这一条填好交
//! `decide`。要问判官的交给 `judges`（O-23 下），判官回来再记判断（`judged.rs`）；别的当场记一笔判断
//! （`ext.onebot.chat.decided`），然后照分派开一轮（`lanes`，施工 O-36）或者回一句提示。
//!
//! - 发的人是谁：终端管理员照核心记下的 `by`（投影），白名单成员照握手交来的 `onebot.whitelist`，别的是别人（第 3 条）。
//! - 重发的（序号在收留着的推送以前就在投影里了）不再判（第 8 条）。
//! - 三个命令编号照判的最后一条的命令编号加 `/decided`、`/respond`、`/queued` 拼（「施工时定的」第 77、99 条）：同一条消息
//!   至多一次判断、一次开回合、一次提示；先记判断、再开回合（`chat.md` 第七条第 4 条）。
//! - 顶替重判的（O-23 下，第 11 条）：先放下在判的那一次（回来的回答丢掉），几条一起走。

use std::time::Instant;

use miyu_chat::{
    Clock, Ctx, Facts, Mode, Moderation, Params, Pending, Said, Standing, Status, Supersede, Venue,
    VenueKind, addressed,
};
use miyu_client::ids::{ExternalId, Seq};
use serde_json::{Value, json};

use super::ask::Asking;
use super::body::{Finale, Judged, body, finale};
use super::decide::{Case, Conclusion, Decision, decide};
use super::deferred::Deferred;
use super::discipline::Discipline;
use super::judges::{Judging, Tag};
use super::projection::DECIDED;
use super::{Route, applied};
use crate::TARGET;
use crate::core::hooks::{Steer, Steered};
use crate::core::{Gone, reason};
use crate::onebot::MediaKind;

/// 禁言时判了要回、排着的那一次，判断的 `outcome` 记成它（施工 O-50）：投影不把它算进判过要回、还没回完的。
const DEFERRED: &str = "deferred";

/// 判一条群消息要的、从这条消息来的几样（`group.rs` 填）。
pub(super) struct Heard {
    /// 命令编号（第 8 条）：判断、开回合、提示的编号照它拼。
    pub(super) id: String,
    /// 平台的消息编号：只记运行日志。
    pub(super) number: i64,
    /// 这个群。
    pub(super) venue: Venue,
    /// 发的人。
    pub(super) sender: ExternalId,
    /// 正文：交给核心的那一份（@ 已经写成名字）。
    pub(super) text: String,
    /// @ 了她。
    pub(super) mentions_me: bool,
    /// @ 了别人。
    pub(super) mentions_others: bool,
    /// 引用的那一条的平台编号。
    pub(super) reply_to: Option<String>,
    /// 没有字：正文只有空白和占位、没有 @（施工 O-37，`Segments::wordless`）。
    pub(super) wordless: bool,
    /// 带的东西的种类，照先后。
    pub(super) media: Vec<MediaKind>,
}

/// 判过的一条：判断、发的人是谁，和判的时候套的参数、此刻（交给判官的照它们）。
struct Weighed {
    /// 判断。
    decision: Decision,
    /// 发的人是谁。
    standing: Standing,
    /// 这时套到这个群上的参数。
    params: Params,
    /// 此刻：在判的这一条的时刻投影里没有时（照说不会）照它。
    clock: Clock,
}

impl Route {
    /// 群会话 `session` 里刚记下的、序号是 `seq` 的这一条（`heard`）：判它，照进站挂接点的意见 `steered` 改（施工 O-43 中，
    /// [`steer`]）；要问判官的交出去，别的记判断、照结论做（第 3 到 8、10、11 条）。`known` 是刚记下的那一刻它是不是已经在投影里
    /// （[`Route::known`]，重发的不再判）：问挂接点的这段时间投影会补上它，所以由调的一方在记下时就看好交进来。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn called(
        &mut self,
        session: &str,
        seq: u64,
        heard: Heard,
        (steered, known): (Steered, bool),
    ) -> Result<(), Gone> {
        self.catch_up(session).await?;
        if known {
            tracing::debug!(target: TARGET, venue = %heard.venue.id(), message = heard.number, "already decided");
            return Ok(());
        }
        let Some(weighed) = self.weigh(session, seq, &heard) else {
            return Ok(());
        };
        let weighed = steer(weighed, steered);
        // 顶替重判的：先放下在判的那一次，它的正文接在前面（几条一起解 base64）。
        let mut texts = Vec::new();
        if let Some(passed) = &weighed.decision.passed
            && let Supersede::Rejudge { cancel, .. } = &passed.supersede
            && let Some(cancelled) = self.judges.cancel(session, cancel.get())
        {
            texts = cancelled.texts;
        }
        texts.push(heard.text.clone());
        match weighed.decision.conclusion {
            Conclusion::Judge(mode) => {
                self.ask(session, (seq, &heard), texts, weighed, mode);
                Ok(())
            }
            _ => {
                let (decision, standing) = (weighed.decision, weighed.standing);
                let who = (standing, &heard.sender);
                self.settle(session, &decision, who, None, &tag(&heard))
                    .await
            }
        }
    }

    /// 群会话 `session` 里序号是 `seq` 的这一条这时是不是已经在投影里：在的是重发的（第 8 条），不再判。要在刚记下、还没收留着
    /// 的推送时看。
    pub(super) fn known(&self, session: &str, seq: u64) -> bool {
        self.groups
            .get(session)
            .is_some_and(|group| group.knows(seq))
    }

    /// 记一笔判断（`judged` 是问了判官的那一次），照最后怎么做做（第 7 条）：回的开一轮，别的不再做什么。发的人是 `sender`，
    /// 是谁是 `standing`。她这时被禁言着的，回的不开轮、排着（施工 O-50，`deferred`），判断的 `outcome` 记成 `deferred`。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn settle(
        &mut self,
        session: &str,
        decision: &Decision,
        (standing, sender): (Standing, &ExternalId),
        judged: Option<Judged<'_>>,
        tag: &Tag,
    ) -> Result<(), Gone> {
        let finale = finale(decision, judged);
        let deferring = finale == Finale::Reply && self.deferring(session);
        let outcome = if deferring { DEFERRED } else { finale.name() };
        tracing::info!(target: TARGET, venue = %tag.venue, message = tag.number, outcome, "chat decided");
        let decided = format!("{}/decided", tag.id);
        let mut written = body(decision, standing, judged);
        written["outcome"] = json!(outcome);
        self.append(session, Some(&decided), DECIDED, written)
            .await?;
        match finale {
            Finale::Reply if deferring => {
                let deferred = Deferred {
                    sender: sender.clone(),
                    standing,
                    decision: decision.clone(),
                    tag: tag.clone(),
                };
                self.defer(session, deferred);
                Ok(())
            }
            Finale::Reply => {
                // 照分派开一轮、并进一轮（施工 O-36，`lanes`），成了的贴表情（施工 O-25 下，「贴表情」第 1 条）。
                if self.dispatched(session, &decision.msgs, tag).await? {
                    self.react(session, decision);
                }
                Ok(())
            }
            Finale::Record => Ok(()),
        }
    }

    /// 交给判官（第 12 条），问的是 `mode`：记下在判的（顶替看它），起一个问判官的任务。`seq`、`heard` 是这一条，`texts` 是
    /// 判的几条的正文。
    fn ask(
        &mut self,
        session: &str,
        (seq, heard): (u64, &Heard),
        texts: Vec<String>,
        weighed: Weighed,
        mode: Mode,
    ) {
        let Weighed {
            decision,
            standing,
            params,
            clock,
        } = weighed;
        let (Some(msg), Some(passed)) = (Seq::new(seq), &decision.passed) else {
            return;
        };
        let absorbed = decision
            .msgs
            .iter()
            .filter(|one| **one != seq)
            .filter_map(|one| Seq::new(*one))
            .collect();
        let at = self
            .groups
            .get(session)
            .and_then(|group| group.at(seq))
            .unwrap_or(clock.now);
        let pending = Pending {
            msg,
            absorbed,
            sender: heard.sender.clone(),
            at,
            status: Status::Judging,
            conditions: passed.conditions.clone(),
        };
        let asking = Asking {
            session: session.to_string(),
            msg: seq,
            mode,
            decoded: params.base64.reveal(&texts.join("\n")),
            judge: params.judge.clone(),
            chatty: params.chatty.clone(),
            // 这个群关了判官的人格的不带、不读（施工 O-23 补，第 12 条第 3 款）。
            persona: self
                .judges
                .persona(session)
                .filter(|_| params.judge.persona)
                .map(str::to_string),
        };
        let judging = Judging {
            session: session.to_string(),
            pending,
            texts,
            decision,
            standing,
            tag: tag(heard),
            chatty: params.chatty,
        };
        self.judges.start(judging, asking);
    }

    /// 照场所规则、投影、在判的、这一条填好交 `decide`（第 3 到 6、10、11、14 条）：交回判断、发的人是谁，和这时套的参数、此刻。
    /// 这个群没有投影的（订阅不上，照说不会）、本机的钟读不出的（记一行）不判。
    fn weigh(&mut self, session: &str, seq: u64, heard: &Heard) -> Option<Weighed> {
        let (group, msg) = (self.groups.get(session)?, Seq::new(seq)?);
        let Some(clock) = applied::clock() else {
            tracing::warn!(target: TARGET, venue = %heard.venue.id(), message = heard.number, "clock not readable, not decided");
            return None;
        };
        let loaded = self.rules.current(Instant::now());
        let applied = loaded.at(&heard.venue);
        let moderation = Moderation {
            keywords: loaded.keywords.clone(),
            base64: applied.params.base64,
        };
        let standing = if group.admin(seq) {
            Standing::Admin
        } else if self.whitelisted(&heard.sender) {
            Standing::Whitelisted
        } else {
            Standing::Member
        };
        let quotes_me = heard.reply_to.as_deref().is_some_and(|msg| group.mine(msg));
        let keywords = applied::keywords(&applied);
        let addressed = addressed(
            VenueKind::Group,
            heard.mentions_me,
            quotes_me,
            &heard.text,
            &keywords,
        );
        // 只有带的东西、没有字（「施工时定的」第 79 条；没有字 O-37 改：原来只有都是表情的算）。
        let media_only = heard.text.trim().is_empty() && !heard.media.is_empty();
        let facts = Facts {
            venue: heard.venue.id().clone(),
            msg,
            said: Said {
                sender: heard.sender.clone(),
                standing,
                addressed,
            },
            mentions_others: heard.mentions_others,
            quotes_other: heard.reply_to.is_some() && !quotes_me,
            textless: heard.wordless,
            media_only,
        };
        let ctx = Ctx {
            rate: applied::rate(&applied),
            sleep: applied::sleep(&applied),
            allow: applied::allow(&applied),
            // 她被禁言着照投影（施工 O-25 中，「群里怎么叫她」第 5 条）；禁言以后 `mute_window` 以内的照常判，判了要回的排着
            // （施工 O-50，`deferred`）。
            muted: group.muted_since(clock.now).is_some_and(|since| {
                clock.now.unix_millis() - since.unix_millis()
                    >= i64::try_from(self.mute_window.as_millis()).unwrap_or(i64::MAX)
            }),
            turns: group.turns(&self.whitelist),
            moderation,
        };
        let pendings: Vec<Pending> = group
            .committed()
            .chain(self.judges.pendings(session))
            .cloned()
            .collect();
        let decision = decide(&Case {
            facts,
            text: heard.text.clone(),
            ctx,
            replies: group.replies(),
            clock,
            chatty: &applied.params.chatty,
            discipline: Discipline::read(applied::discipline(&applied)),
            pendings: &pendings,
            window: applied.params.supersede_window,
        });
        Some(Weighed {
            decision,
            standing,
            params: applied.params,
            clock,
        })
    }

    /// 往会话 `session` 记一条 `kind` 的事件（`events.append`）：命令编号是 `id`，空的自己编。交回记成的序号（施工 O-25 中：
    /// 入队照它往下走）；被拒的（照说不会）、回应里没有序号的（协议不对）记一行 `WARN`，交回空的。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn append(
        &mut self,
        session: &str,
        id: Option<&str>,
        kind: &str,
        body: Value,
    ) -> Result<Option<u64>, Gone> {
        let params = json!({"session": session, "kind": kind, "body": body});
        let reply = match id {
            Some(id) => self.core.call_as(id, "events.append", params).await?,
            None => self.core.call("events.append", params).await?,
        };
        if let Some(reason) = reason(&reply) {
            tracing::warn!(target: TARGET, session, kind, reason, "event not appended");
            return Ok(None);
        }
        let seq = reply["result"]["seq"].as_u64();
        if seq.is_none() {
            tracing::warn!(target: TARGET, session, kind, "event appended without a seq");
        }
        Ok(seq)
    }
}

/// 这一条的命令编号、平台编号、场所：判断、开一轮的命令编号照它拼，运行日志照它记。
fn tag(heard: &Heard) -> Tag {
    Tag {
        id: heard.id.clone(),
        number: heard.number,
        venue: heard.venue.id().to_string(),
    }
}

/// 照进站挂接点的意见改判断（施工 O-43 中，`onebot.md`「挂接点」第 3 条）：只记下的改成只记下，终端管理员的不改；直接回的，进站
/// 链放行了的改成回（限流、睡觉、不让叫她的群在进站链就只记下了，越不过去；放行了、额度满了的只有不受限流的白名单成员，照他们
/// 的豁免回）；照常的、改不了的不动。改了的把意见记在判断上（`body` 写 `hook`）。
fn steer(mut weighed: Weighed, steered: Steered) -> Weighed {
    let decision = &mut weighed.decision;
    let to = match steered.steer {
        Steer::Record if weighed.standing != Standing::Admin => Conclusion::Record,
        Steer::Reply if decision.passed.is_some() => Conclusion::Reply,
        _ => return weighed,
    };
    if decision.conclusion != to {
        decision.conclusion = to;
        decision.steered = Some(steered);
    }
    weighed
}
