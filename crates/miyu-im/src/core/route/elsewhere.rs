//! 终端、网页里用 im 的场所工具，和终端管理员叫她去别的场所办（设计 13）。
//!
//! - 本机的会话（终端、网页）里的调用（[`Route::local_called`]）：主动发消息照驱动的 `ToolSet::local` 办（施工 I-40）；别的登记了
//!   `local` 的那几件照参数 `group`、`private` 找那个场所（`ToolSet::reach`：通讯录、头像不找，查聊天记录不写是查全部），照那个
//!   场所的开关算给不给（驱动的功能开关、场所插件），在那个场所里办，照终端管理员算（`admin`、`commanded`），答的话前头写是哪个场所。
//! - 群、私聊里写了别的场所的（`driven.rs`）：只有终端管理员直接叫她的那一轮照上面那样去办，别人叫的答 `only-here`。
//! - 那个场所 im 还没有会话的（没说过话的群）照样办：查看的几件只要机器人号的连接和场所的号。

use std::sync::Arc;

use serde_json::Value;

use super::Route;
use super::outreach::Answer;
use super::session::Peer;
use crate::TARGET;
use crate::chat::{Id, Platform, To};
use crate::core::{Answerer, Gone};
use crate::rules::Toolset;
use crate::toolset::{Effect, Planned, Reach, ToolSet};
use miyu_chat::VenueKind;

/// 一次调用：编号、哪件工具、哪条线、答它的口子。
pub(super) struct Asked {
    pub(super) id: Value,
    pub(super) tool: String,
    pub(super) line: String,
    pub(super) answerer: Answerer,
}

/// 参数里写的场所。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Aimed {
    /// 没写（或者这件工具不认这两格）。
    Here,
    /// 写了一个。
    At(To),
    /// 两个都写了、写的不是号。
    Bad,
}

/// 这一次 `args` 写了哪个场所（纯逻辑）：只认终端里能用、要说场所的那几件（`venues` 带 `local`、`reach` 不是 `Anywhere`），参数
/// `group`、`private` 至多一个，号照整数或者数字的字（Telegram 的群号是负的，施工 I-220）。别的工具（自己的 `group` 另有意思的，如头像、发消息）是 `Here`。
pub(super) fn aimed(set: &dyn ToolSet, tool: &str, args: &Value) -> Aimed {
    let local = set
        .tools()
        .iter()
        .any(|decl| decl.name == tool && decl.venues.contains(&"local"));
    if !local || set.reach(tool) == Reach::Anywhere {
        return Aimed::Here;
    }
    let number = |value: &Value| -> Result<Option<Id>, ()> {
        match value {
            Value::Null => Ok(None),
            Value::Number(number) => number
                .as_i64()
                .filter(|n| *n != 0)
                .map(|n| Some(Id::from(n)))
                .ok_or(()),
            Value::String(text) => Id::new(text.trim())
                .filter(|id| id.number().is_some_and(|n| n != 0))
                .map(Some)
                .ok_or(()),
            _ => Err(()),
        }
    };
    match (number(&args["group"]), number(&args["private"])) {
        (Ok(None), Ok(None)) => Aimed::Here,
        (Ok(Some(group)), Ok(None)) => Aimed::At(To::Group(group)),
        (Ok(None), Ok(Some(user))) => Aimed::At(To::Private(user)),
        _ => Aimed::Bad,
    }
}

impl Route {
    /// 本机的会话里的一次调用（模块的说明）。
    ///
    /// # Errors
    ///
    /// 写不出去、核心断开。
    pub(super) async fn local_called(
        &mut self,
        params: &Value,
        asked: Asked,
        toolset: Toolset,
    ) -> Result<(), Gone> {
        let (platform, set, answers) = &toolset;
        // 主动发消息（施工 I-40）：经参数 `bot` 写的号，没写的这个平台只连着一个号时经它（施工 I-81 下下）。
        match set.local(&asked.tool, &params["args"], answers) {
            Some(Planned::Host(Effect::Send { to, text })) => {
                let bot = match self.local_sender(*platform, &params["args"]["bot"]) {
                    Ok(bot) => bot,
                    Err(bots) => {
                        tracing::info!(target: TARGET, session = asked.line, why = "which-bot", "platform tool failed");
                        let said = answers.say("which-bot", &[("bots", &bots)], true);
                        asked.answerer.answer(&asked.id, said);
                        return Ok(());
                    }
                };
                let answer = Answer::Tool(asked.id, Arc::clone(answers), asked.answerer);
                return self.reach((*platform, bot), (to, text), answer).await;
            }
            Some(Planned::Now(answer)) => {
                asked.answerer.answer(&asked.id, answer);
                return Ok(());
            }
            _ => {}
        }
        let local = set
            .tools()
            .iter()
            .any(|decl| decl.name == asked.tool && decl.venues.contains(&"local"));
        if !local {
            tracing::warn!(target: TARGET, session = asked.line, tool = asked.tool, why = "unreachable", "platform tool failed");
            asked
                .answerer
                .answer(&asked.id, answers.say("unreachable", &[], true));
            return Ok(());
        }
        let target = match (
            set.reach(&asked.tool),
            aimed(set.as_ref(), &asked.tool, &params["args"]),
        ) {
            (Reach::Anywhere, _) | (Reach::Either, Aimed::Here) => None,
            (_, Aimed::Bad) => {
                asked
                    .answerer
                    .answer(&asked.id, answers.say("chat-bad", &[], true));
                return Ok(());
            }
            (Reach::Chat, Aimed::Here) => {
                asked
                    .answerer
                    .answer(&asked.id, answers.say("chat-needed", &[], true));
                return Ok(());
            }
            (_, Aimed::At(to)) => Some(to),
        };
        self.elsewhere(params, asked, toolset, (target, None)).await
    }

    /// 照终端管理员算，在场所 `target`（空的：不找场所，经机器人号 `bot`，没给的照这个平台连着的挑）里办这一次调用（模块的说明）。
    ///
    /// # Errors
    ///
    /// 写不出去、核心断开。
    pub(super) async fn elsewhere(
        &mut self,
        params: &Value,
        asked: Asked,
        toolset: Toolset,
        (target, bot): (Option<To>, Option<Id>),
    ) -> Result<(), Gone> {
        let (platform, set, answers) = &toolset;
        let Some(bot) = bot.or_else(|| self.bot_for(*platform, target.as_ref())) else {
            tracing::warn!(target: TARGET, session = asked.line, tool = asked.tool, why = "not_connected", "platform tool failed");
            asked
                .answerer
                .answer(&asked.id, answers.say("unreachable", &[], true));
            return Ok(());
        };
        // 不找场所的（通讯录、头像、查全部聊天记录）借机器人号自己的私聊当场所：这几件不看它。
        let to = target.clone().unwrap_or_else(|| To::Private(bot.clone()));
        let Some(venue) = venue_of(*platform, &bot, &to) else {
            asked
                .answerer
                .answer(&asked.id, answers.say("chat-bad", &[], true));
            return Ok(());
        };
        // 照那个场所的开关（设计 13 第一节第 3 条）：驱动的功能开关，插件在那个场所开着没有。
        let config = self.hooks.driver_config(*platform);
        let plugged = || {
            set.plugin(&asked.tool).is_none_or(|plugin| {
                self.hooks
                    .active(*platform, Some(&bot), to.clone())
                    .contains(&plugin)
            })
        };
        if !set.offers(&asked.tool, &config) || (target.is_some() && !plugged()) {
            tracing::info!(target: TARGET, venue = %venue, tool = asked.tool, why = "off", "platform tool failed");
            asked
                .answerer
                .answer(&asked.id, answers.say("chat-off", &[], true));
            return Ok(());
        }
        let label = match &target {
            Some(To::Group(group)) => answers.text("chat-in-group", &[("group", group.as_str())]),
            Some(To::Private(user)) => answers.text("chat-in-private", &[("user", user.as_str())]),
            None => String::new(),
        };
        let session = self
            .venues
            .get(venue.as_str())
            .cloned()
            .unwrap_or_else(|| asked.line.clone());
        let peer = Peer {
            platform: *platform,
            bot,
            to,
            venue,
        };
        let mut params = params.clone();
        if target.is_none() && set.reach(&asked.tool) == Reach::Either {
            // 查聊天记录不写场所：查全部（终端管理员才能，这里就是）。
            params["args"]["all_chats"] = Value::Bool(true);
        }
        self.venue_called(&params, asked, (session, peer), toolset, Some(label))
            .await
    }

    /// 平台 `platform` 上办这一次挑哪个机器人号：场所 `target` 已经有会话的那个号优先，没有的照连着的头一个；一个都没连着的是空的。
    fn bot_for(&self, platform: Platform, target: Option<&To>) -> Option<Id> {
        let connected: Vec<Id> = self
            .platforms
            .bots(platform)
            .held()
            .into_iter()
            .map(|(bot, ..)| bot)
            .collect();
        let known = target.and_then(|to| {
            connected.iter().find(|bot| {
                venue_of(platform, bot, to)
                    .is_some_and(|venue| self.venues.contains_key(venue.as_str()))
            })
        });
        known.or(connected.first()).cloned()
    }
}

/// 机器人号 `bot` 收发的场所 `to` 的编号；号写不成编号的是空的。
fn venue_of(platform: Platform, bot: &Id, to: &To) -> Option<miyu_client::ids::VenueId> {
    let (kind, number) = match to {
        To::Group(group) => (VenueKind::Group, group),
        To::Private(user) => (VenueKind::Private, user),
    };
    platform
        .venue(bot, kind, number)
        .ok()
        .map(|venue| venue.id().clone())
}

#[cfg(test)]
mod tests;
