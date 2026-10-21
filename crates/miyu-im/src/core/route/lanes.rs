//! 分派和支线（施工 O-36，`onebot.md` 第一条「群里怎么叫她」第 1、6、7、9 条；`chat.md` 第四条；`session/fork.md`）：判下来要回
//! 的照群聊内核的 `dispatch` 做，支线推来的事件收进主线的投影。纯逻辑的那一半在 `projection/lanes.rs`。
//!
//! - 回对人（施工 I-21）：开轮、并进都带一块「你只回这几条，别的另有安排」（模板是出厂数据，`rules/facts.rs`，[`Route::own_line`]）：
//!   几个人同时叫她时，原来主线什么都不带、支线只写一条别的线，她把别人的题一起答（实测 6 轮全有）；改了以后答别人的题 1%。
//! - `StartMain`：`session.respond` 主线。
//! - `JoinMain`：`session.respond` 主线。
//! - `Queue`：排在这个群会话后面，主线这一轮结束了开下一轮（施工 I-16 照旧版，`waiting`；原来并进主线这一轮）。
//! - `JoinLane`：判的几条里比分叉点晚、还没抄进去的先抄（`session.send`，命令编号加 `/branch`，`venue` 带 `ambient`），再
//!   `session.respond` 支线。
//! - `Fork`：`session.fork {seal_after: 1}`、订阅子会话（`after` 是分叉点）、把留着的人话里比分叉点晚的照先后抄进去，最后
//!   `session.respond` 支线。分叉被拒的照 `Queue` 办；订阅不上、开不了轮的支线当场去掉（不然它不会
//!   封存，一直占着位置）。
//! - 覆盖打断重来（施工 I-21，2026-10-21 项目主人：「相当于在终端里发错了、打断重新编辑再请求」）：顶替接过去的（[`Route::redone`]），
//!   在回前一条的那条线这一轮还一段话都没入队的，`session.interrupt` 那条线（排着队的留着），再照这几条开一轮：核心不让已经当过
//!   触发的再当（`already_answered`），所以 `to` 只交新的这几条，「你只回这几条」写全。已经说过话的照旧并进去。实测（旧版也这样）：
//!   补发的那条并进正在跑的一轮，那次请求已经发出去，她先回一句过时的再答。
//! - 支线刚好封存了（`session_sealed`）：先把留着的推送收进来（封存的那一条在里面，支线跟着去掉），重新分派一次，只重来一次。
//! - 支线推来的（[`Route::lane_heard`]）：换回主线序号收进主线的投影，贴着的表情照 `(line, turn)` 记、摘，「不说话」照子会话
//!   编号认，她的话照主线的出站链发；`session.sealed` 退订。

use miyu_chat::{Dispatch, Supersede, Venue, dispatch};
use miyu_client::events::Event;
use serde_json::{Value, json};

use super::decide::Decision;
use super::judges::Tag;
use super::projection::{Carry, Seen};
use super::voicing::Voice;
use super::{Route, applied, reply_text};
use crate::TARGET;
use crate::core::{Gone, reason};

/// 「你只回这几条」那一块事实的 `kind`（「群里怎么叫她」第 7 条「事实」，施工 I-21 改字）。
const ANOTHER_LINE: &str = "another-line";

/// 拒绝码：支线封存了（`session/fork.md`「封存」）。
const SEALED: &str = "session_sealed";

/// 开一轮、并进一轮怎么样了。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Responded {
    /// 开了一轮、并进了一轮。
    Asked,
    /// 被拒了（记了运行日志）。
    Refused,
    /// 那条支线已经封存了：重新分派。
    Sealed,
}

impl Route {
    /// 群会话 `session` 里判下来要回的几条 `msgs`（最后一条的命令编号、平台编号、场所是 `tag`）照分派做（第 6、7 条）：交回开没
    /// 开成（成了的贴表情）。支线刚好封存了的收进留着的推送、重新分派一次。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn dispatched(
        &mut self,
        session: &str,
        msgs: &[u64],
        tag: &Tag,
    ) -> Result<bool, Gone> {
        for retried in [false, true] {
            match self.dispatch_once(session, msgs, tag).await? {
                Responded::Asked => return Ok(true),
                Responded::Refused => return Ok(false),
                Responded::Sealed if !retried => self.catch_up(session).await?,
                Responded::Sealed => {
                    tracing::warn!(target: TARGET, venue = %tag.venue, message = tag.number, reason = SEALED, "respond refused");
                }
            }
        }
        Ok(false)
    }

    /// 分派一次：照投影的 `Lines`、这个群的 `parallel` 问群聊内核，照结论做。发的人认不出的（照说不会）照并进主线办。
    async fn dispatch_once(
        &mut self,
        session: &str,
        msgs: &[u64],
        tag: &Tag,
    ) -> Result<Responded, Gone> {
        let parallel = match self
            .peers
            .get(session)
            .and_then(|peer| Venue::parse(&peer.venue))
        {
            Some(venue) => applied::parallel(&self.applied(&venue)),
            None => 0,
        };
        let Some(group) = self.groups.get(session) else {
            return self.respond(session, msgs, &[], tag).await;
        };
        let lines = group.lines(parallel);
        let decided = match msgs.last().and_then(|seq| group.sender(*seq)) {
            Some(sender) => dispatch(sender, &lines),
            None => Dispatch::JoinMain,
        };
        let facts = self.own_line(session, msgs);
        match decided {
            Dispatch::StartMain | Dispatch::JoinMain => {
                self.respond(session, msgs, &facts, tag).await
            }
            Dispatch::Queue => Ok(if self.wait(session, msgs, tag) {
                Responded::Asked
            } else {
                Responded::Refused
            }),
            Dispatch::JoinLane(index) => match group.lane_at(index).map(str::to_string) {
                Some(lane) => self.join_lane(session, &lane, msgs, &facts, tag).await,
                None => self.respond(session, msgs, &facts, tag).await,
            },
            Dispatch::Fork => self.fork(session, msgs, tag).await,
        }
    }

    /// 覆盖打断重来（模块的说明）：`decision` 是顶替接过去的、回前一条的那条线这一轮还没说过话的，打断它、照几条开一轮，交回开成了
    /// 没有；不是这种的（没顶替、说过话了、核心说没在跑）交回假，照常分派。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn redone(
        &mut self,
        session: &str,
        decision: &Decision,
        tag: &Tag,
    ) -> Result<bool, Gone> {
        let Some(Supersede::Inherit { msg, .. }) =
            decision.passed.as_ref().map(|passed| &passed.supersede)
        else {
            return Ok(false);
        };
        let Some((line, mut msgs)) = self
            .groups
            .get(session)
            .and_then(|group| group.redo(msg.get()))
        else {
            return Ok(false);
        };
        let params = json!({"session": line, "queued": "keep"});
        let reply = self.core.call("session.interrupt", params).await?;
        if let Some(reason) = reason(&reply) {
            tracing::debug!(target: TARGET, venue = %tag.venue, message = tag.number, reason, "turn not interrupted for a supersede");
            return Ok(false);
        }
        tracing::info!(target: TARGET, venue = %tag.venue, message = tag.number, line, "turn interrupted for a supersede");
        for seq in &decision.msgs {
            if !msgs.contains(seq) {
                msgs.push(*seq);
            }
        }
        let facts = self.own_line(session, &msgs);
        let responded = match line == session {
            true => self.respond(session, &decision.msgs, &facts, tag).await?,
            false => {
                self.join_lane(session, &line, &decision.msgs, &facts, tag)
                    .await?
            }
        };
        Ok(responded == Responded::Asked)
    }

    /// 并进群会话 `session` 的支线 `lane`：判的几条 `msgs` 里还没抄进去的先抄，再 `session.respond` 支线，带上事实 `facts`。
    async fn join_lane(
        &mut self,
        session: &str,
        lane: &str,
        msgs: &[u64],
        facts: &[Value],
        tag: &Tag,
    ) -> Result<Responded, Gone> {
        let carries = self
            .groups
            .get(session)
            .map(|group| group.to_copy(lane, Some(msgs)))
            .unwrap_or_default();
        for carry in carries {
            if self.carry(session, lane, carry).await? == Responded::Sealed {
                return Ok(Responded::Sealed);
            }
        }
        self.respond_lane(session, lane, msgs, facts, tag).await
    }

    /// 给群会话 `session` 分叉一条支线回 `msgs`（第 7 条 `Fork`）。
    async fn fork(&mut self, session: &str, msgs: &[u64], tag: &Tag) -> Result<Responded, Gone> {
        let id = format!("{}/fork", tag.id);
        let params = json!({"session": session, "seal_after": 1});
        let reply = self.core.call_as(&id, "session.fork", params).await?;
        let result = &reply["result"];
        let (Some(lane), Some(upto)) = (result["session"].as_str(), result["upto"].as_u64()) else {
            let reason = reason(&reply).unwrap_or("bad_reply");
            tracing::warn!(target: TARGET, venue = %tag.venue, message = tag.number, reason, "lane not forked");
            return self.respond(session, msgs, &[], tag).await;
        };
        let lane = lane.to_string();
        let Some(group) = self.groups.get_mut(session) else {
            return Ok(Responded::Refused);
        };
        group.forked(&lane, upto, msgs);
        tracing::info!(target: TARGET, venue = %tag.venue, message = tag.number, lane, upto, "lane forked");
        let facts = self.own_line(session, msgs);
        // 群名那一段照主线的给支线也放一份（施工 O-49）：开轮以前放好，支线第一次请求的 system 和主线的一样。
        self.title_lane(session, &lane).await?;
        let responded = match self.follow_lane(&lane, upto).await? {
            true => {
                self.catch_up(session).await?;
                let carries = self
                    .groups
                    .get(session)
                    .map(|group| group.to_copy(&lane, None))
                    .unwrap_or_default();
                for carry in carries {
                    self.carry(session, &lane, carry).await?;
                }
                self.respond_lane(session, &lane, msgs, &facts, tag).await?
            }
            false => Responded::Refused,
        };
        if responded != Responded::Asked {
            // 开不了轮的支线不会封存：去掉、退订，不让它一直占着位置。
            if let Some(group) = self.groups.get_mut(session) {
                group.dropped(&lane);
            }
            self.unfollow(&lane).await?;
        }
        Ok(responded)
    }

    /// 把一条人话 `carry` 抄进群会话 `session` 的支线 `lane`（第 7 条「抄」），记进序号对照表。核心拒了的记一行：封存了的交回
    /// `Sealed`，别的照样往下走（`respond` 里就没有这一条）。
    async fn carry(&mut self, session: &str, lane: &str, carry: Carry) -> Result<Responded, Gone> {
        let mut params = carry.params;
        params["session"] = json!(lane);
        let reply = self.core.call_as(&carry.id, "session.send", params).await?;
        match (reason(&reply), reply["result"]["events"][0].as_u64()) {
            (None, Some(at)) => {
                if let Some(group) = self.groups.get_mut(session) {
                    group.copied(lane, at, carry.msg);
                }
                Ok(Responded::Asked)
            }
            (Some(SEALED), _) => Ok(Responded::Sealed),
            (reason, _) => {
                let reason = reason.unwrap_or("bad_reply");
                tracing::warn!(target: TARGET, session, lane, reason, "copy refused");
                Ok(Responded::Refused)
            }
        }
    }

    /// 支线 `lane` 照判的几条 `msgs`（主线序号）开一轮、并进一轮：换成支线里的序号，带上事实 `facts`。一条都换不出来的（照说
    /// 不会：都抄过了）记一行，不开。
    async fn respond_lane(
        &mut self,
        session: &str,
        lane: &str,
        msgs: &[u64],
        facts: &[Value],
        tag: &Tag,
    ) -> Result<Responded, Gone> {
        let to = self
            .groups
            .get(session)
            .map(|group| group.targets(lane, msgs))
            .unwrap_or_default();
        if to.is_empty() {
            tracing::warn!(target: TARGET, venue = %tag.venue, message = tag.number, lane, "nothing to respond on the lane");
            return Ok(Responded::Refused);
        }
        self.respond(lane, &to, facts, tag).await
    }

    /// 照记下的几条 `to` 在会话 `target`（主线或者支线）开一轮（第 7 条）：`session.respond {session, to, facts?}`，命令编号照
    /// 最后一条的 `tag` 加 `/respond`；正在跑一轮的核心并进去。核心说已经当过触发、不是旁听的不再开，记一行；封存了的交回
    /// `Sealed`，调的一方重新分派。
    async fn respond(
        &mut self,
        target: &str,
        to: &[u64],
        facts: &[Value],
        tag: &Tag,
    ) -> Result<Responded, Gone> {
        let id = format!("{}/respond", tag.id);
        let mut params = json!({"session": target, "to": to});
        // 为什么叫她（施工 I-16，`primer`）跟在别的事实后面。
        let facts: Vec<&Value> = facts.iter().chain(&tag.facts).collect();
        if !facts.is_empty() {
            params["facts"] = json!(facts);
        }
        let reply = self.core.call_as(&id, "session.respond", params).await?;
        let (venue, number) = (&tag.venue, tag.number);
        match reason(&reply) {
            None => {
                tracing::info!(target: TARGET, venue = %venue, message = number, session = target, "respond asked");
                Ok(Responded::Asked)
            }
            Some(SEALED) => Ok(Responded::Sealed),
            Some(reason @ ("already_answered" | "not_ambient")) => {
                let messages = &reply["error"]["data"]["messages"];
                tracing::info!(target: TARGET, venue = %venue, message = number, reason, messages = %messages, "respond refused");
                Ok(Responded::Refused)
            }
            Some(reason) => {
                tracing::warn!(target: TARGET, venue = %venue, message = number, reason, "respond refused");
                Ok(Responded::Refused)
            }
        }
    }

    /// 群会话 `session` 里这一轮回的几条 `msgs`（主线序号）写一块「你只回这几条，别的另有安排」（施工 I-21）：照先后写平台编号
    /// （`msg=1, msg=2`）。一条平台编号都没有的（照说不会）不带；写不出的（照说不会：起来时查过字段）记一行、不带。
    fn own_line(&self, session: &str, msgs: &[u64]) -> Vec<Value> {
        let Some(group) = self.groups.get(session) else {
            return Vec::new();
        };
        let ids: Vec<String> = msgs
            .iter()
            .filter_map(|seq| group.msg(*seq))
            .map(|msg| format!("msg={msg}"))
            .collect();
        if ids.is_empty() {
            return Vec::new();
        }
        let joined = ids.join(", ");
        let fields = [("msgs", joined.as_str())].into_iter().collect();
        match self.rules.another_line().render(&fields) {
            Ok(text) => vec![json!({"kind": ANOTHER_LINE, "text": text})],
            Err(error) => {
                tracing::warn!(target: TARGET, error = %error, "own line not written");
                Vec::new()
            }
        }
    }

    /// 订阅支线 `lane`，补 `after` 以后的（第 1 条：分叉点、掉了队收到的最后一条）。订阅不上的记一行、交回假。
    async fn follow_lane(&mut self, lane: &str, after: u64) -> Result<bool, Gone> {
        let params = json!({"session": lane, "stream": "events", "after": after});
        let reply = self.core.call("subscribe", params).await?;
        if let Some(reason) = reason(&reply) {
            tracing::warn!(target: TARGET, session = lane, reason, "not subscribed");
            return Ok(false);
        }
        Ok(true)
    }

    /// 退订支线 `lane`（封存了、用不成了）。核心拒了的（已经不在了）记一行调试日志。
    async fn unfollow(&mut self, lane: &str) -> Result<(), Gone> {
        let params = json!({"session": lane, "stream": "events"});
        let reply = self.core.call("unsubscribe", params).await?;
        if let Some(reason) = reason(&reply) {
            tracing::debug!(target: TARGET, session = lane, reason, "not unsubscribed");
        }
        Ok(())
    }

    /// 支线 `lane` 是哪个群会话（主线）的；不是谁的支线的是空的。
    pub(super) fn main_of_lane(&self, lane: &str) -> Option<String> {
        self.groups
            .iter()
            .find(|(_, group)| group.has_lane(lane))
            .map(|(session, _)| session.clone())
    }

    /// 会话 `session` 的主线：是支线的交回它的群会话，别的交回它自己（施工 O-36：平台工具照主线找投影、发到哪）。
    pub(super) fn main_of(&self, session: &str) -> String {
        self.main_of_lane(session)
            .unwrap_or_else(|| session.to_string())
    }

    /// 群会话 `session` 的支线 `lane` 推来的一条 `pushed`（`event`；`resync` 照收到的最后一条再订阅）：收进主线的投影，照交回的做
    /// （见模块开头）。
    ///
    /// # Errors
    ///
    /// 她的话入队时、退订时写不出去、等的时候核心断开。
    pub(super) async fn lane_heard(
        &mut self,
        session: &str,
        lane: &str,
        pushed: &Value,
    ) -> Result<(), Gone> {
        if pushed["method"] == "resync" {
            let last = self
                .groups
                .get(session)
                .and_then(|group| group.lane_last(lane));
            if let Some(last) = last {
                self.follow_lane(lane, last).await?;
            }
            return Ok(());
        }
        let raw = &pushed["params"]["event"];
        if raw["seq"].as_u64().is_none() {
            return Ok(());
        }
        let event: Event = match serde_json::from_value(raw.clone()) {
            Ok(event) => event,
            Err(error) => {
                tracing::warn!(target: TARGET, session = lane, error = %error, "event not understood");
                return Ok(());
            }
        };
        let quiet = self.quiet.heard(lane, raw);
        // 她在支线给人看的文件、发出去的图照主线发进群里（施工 O-46，`shown`；O-51，`stickers`），这一轮不说话的不发。
        if !quiet && self.groups.contains_key(session) {
            self.show((session, lane), raw);
            self.stickers((session, lane), raw).await?;
        }
        let Some(group) = self.groups.get_mut(session) else {
            return Ok(());
        };
        match group.lane_take(lane, &event) {
            Seen::Took { turn, msgs } => self.reactions.taken(session, lane, &msgs, turn),
            Seen::Ended(turn) => {
                self.reactions.done(session, lane, turn);
                // 仅发送最终消息时扣着的那段（施工 O-57）。
                self.turn_closed(lane, raw).await?;
            }
            Seen::Spoke(_) if quiet => self.hushed(session, &reply_text(raw)),
            Seen::Spoke(speaking) => {
                let voice = Voice::Group(speaking);
                self.say_or_hold((session, lane), raw, voice, reply_text(raw))
                    .await?;
            }
            Seen::Sealed => {
                tracing::info!(target: TARGET, session, lane, "lane sealed");
                // 这条支线的回合、驱动的工具调了几次不用再记（施工 T-3）。
                self.turns.remove(lane);
                self.uses.retain(|(line, _), _| line != lane);
                self.unfollow(lane).await?;
            }
            Seen::Nothing => {}
        }
        Ok(())
    }
}
