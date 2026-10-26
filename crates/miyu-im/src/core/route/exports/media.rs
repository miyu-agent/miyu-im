//! `chat.media`（施工 I-39，设计 34 第九节：表情包包偷图）：别的包要那个会话所在场所里一条消息的第几样图和它的来历。
//!
//! - 参数 `{session, msg, index?}`：`session` 是调工具的那个会话（核心的 `tool.call` 交给提供者的），`msg` 是平台编号（她看到的
//!   `[msg=…]`，整数或写成整数的字），`index` 是那一条里第几样（她看到的 `#n`，从 1 数，不写是 1）。写错的拒 `bad_params`。
//! - 照她的取图工具取（`get_msg` 现认段、核归属、数第几样、`get_image`）：认不得的会话拒 `unknown_session`；那个号没连着的
//!   `not_connected`；没有这一条、不在这个场所的 `not_found`；没有第几样的 `no_item`；不是图的（视频、文件、语音、小黄脸、商城
//!   表情）`not_image`；QQ 回了失败、等不到、拿不到手、超了图的上限的 `unfetched`。
//! - 答 `{"kind", "data", "name", "origin"}`：`kind` 是 `image` 或 `sticker`，`data` 是图的 base64，`name` 是 QQ 说的文件名（没有的
//!   是 `null`），`origin` 是来历：平台、场所、消息编号、发的人（`id`、`name`，照 im 的聊天记录；没记下的照 QQ 回的号、名字是
//!   `null`）、`at`（毫秒，没记下的是 `null`）。

use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use super::super::Route;
use crate::TARGET;
use crate::chat::{Fetch, Link, MediaKind, Platform, To};
use crate::core::Answerer;
use crate::toolset::{History, Missed, kept, message, msg_id, owned, take, too_large};

/// 读好的一次 `chat.media`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Wanted {
    /// 调工具的会话。
    pub(super) session: String,
    /// 平台编号。
    pub(super) msg: String,
    /// 第几样，从 1 数。
    pub(super) index: usize,
}

impl Wanted {
    /// 读参数（纯逻辑）：`session` 是不空的字，`msg` 照 [`msg_id`]，`index` 不写是 1、写了是正整数。别的写法交回空的。
    pub(super) fn read(args: &Value) -> Option<Wanted> {
        let session = args["session"]
            .as_str()
            .filter(|session| !session.is_empty())?;
        let msg = msg_id(&args["msg"])?;
        let index = match &args["index"] {
            Value::Null => 1,
            index => usize::try_from(index.as_u64()?)
                .ok()
                .filter(|index| *index > 0)?,
        };
        Some(Wanted {
            session: session.to_string(),
            msg,
            index,
        })
    }
}

/// 取的时候要的：平台、连接、场所、对方、等多久、图最多多少字节、聊天记录。
struct Taking {
    platform: Platform,
    link: Arc<dyn Link>,
    venue: String,
    to: To,
    wait: Duration,
    limit: usize,
    history: History,
}

impl Route {
    /// 办一次 `chat.media`（模块的说明）：认会话、找连接当场办，取图另起任务。
    pub(super) fn media_exported(
        &mut self,
        id: Value,
        from: String,
        args: &Value,
        answerer: Answerer,
    ) {
        let Some(wanted) = Wanted::read(args) else {
            tracing::info!(target: TARGET, from, "export media refused");
            answerer.refuse(&id, "bad_params");
            return;
        };
        let main = self.main_of(&wanted.session);
        let Some(peer) = self.peers.get(&main) else {
            answerer.refuse(&id, "unknown_session");
            return;
        };
        let Some(link) = self.bots.get(peer.bot) else {
            answerer.refuse(&id, "not_connected");
            return;
        };
        let taking = Taking {
            platform: self.platform,
            link,
            venue: peer.venue.as_str().to_string(),
            to: peer.to,
            wait: self.fetching.quick,
            limit: self.fetching.image_limit,
            history: self.records.history(),
        };
        self.chores.spawn(async move {
            match fetched(&taking, &wanted).await {
                Ok(answer) => {
                    tracing::info!(target: TARGET, from, msg = wanted.msg, index = wanted.index, "export media");
                    answerer.answer(&id, answer);
                }
                Err(why) => {
                    tracing::info!(target: TARGET, from, msg = wanted.msg, why, "export media failed");
                    answerer.refuse(&id, why);
                }
            }
        });
    }
}

/// 取那一样和它的来历：成了的交回答的内容，没成的交回拒的原因（模块的说明）。
async fn fetched(taking: &Taking, wanted: &Wanted) -> Result<Value, &'static str> {
    let link = taking.link.as_ref();
    let fetched = match message(link, &wanted.msg).await {
        Ok(fetched) => fetched,
        Err(Missed::Failed(_)) => return Err("not_found"),
        Err(Missed::Unanswered | Missed::Unfetched(_)) => return Err("unfetched"),
        Err(Missed::Unreachable) => return Err("not_connected"),
    };
    if !owned(&fetched, taking.to) {
        return Err("not_found");
    }
    let media = kept(&fetched.segments.media);
    let item = wanted
        .index
        .checked_sub(1)
        .and_then(|at| media.get(at))
        .ok_or("no_item")?;
    if item.fetch != Some(Fetch::Image) {
        return Err("not_image");
    }
    if too_large(item, taking.limit) {
        return Err("unfetched");
    }
    let (bytes, named) = take(link, Fetch::Image, &item.id, (taking.wait, taking.limit))
        .await
        .map_err(|missed| match missed {
            Missed::Unreachable => "not_connected",
            _ => "unfetched",
        })?;
    let recorded = taking
        .history
        .sender_of(&taking.venue, &wanted.msg)
        .await
        .unwrap_or_else(|error| {
            tracing::debug!(target: TARGET, error, "export media origin unread");
            None
        });
    let (sender, name, at) = match recorded {
        Some(recorded) => (
            recorded["sender"].clone(),
            recorded["name"].clone(),
            recorded["at"].clone(),
        ),
        None => (
            fetched
                .user
                .and_then(|user| taking.platform.person(user).ok())
                .map_or(Value::Null, |person| json!(person.as_str())),
            Value::Null,
            Value::Null,
        ),
    };
    let kind = if item.kind == MediaKind::Sticker {
        "sticker"
    } else {
        "image"
    };
    Ok(json!({
        "kind": kind,
        "data": STANDARD.encode(&bytes),
        "name": named,
        "origin": {
            "platform": taking.platform.name(),
            "venue": taking.venue,
            "msg": wanted.msg,
            "sender": {"id": sender, "name": name},
            "at": at,
        },
    }))
}

#[cfg(test)]
mod tests;
