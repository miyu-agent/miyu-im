//! 合并转发的展开（施工 O-34，`onebot.md` 第一条「群消息」第 2 条、第 6 条，「施工时定的」第 206 条起）：群消息、私聊里的
//! `forward` 段，交 `session.send` 以前把里面的消息取出来，写进场所的格的 `forwards`（`venues.md`「场所的格」），核心在那一行
//! 下面缩进写（`kernel/request.md`「合并转发」）。正文里的占位 `[forward]` 照旧写。
//!
//! - 怎么取：段里内联了消息的（NapCat 开了解析合并转发，套在里面的也内联）照它；没有的经收进这条消息的那个机器人号现在的连接
//!   调 `get_forward_msg {id}`，等一次 OneBot 调用（`call_timeout_seconds`）。没连着、失败、等不到、回的不是消息的数组：这个
//!   转发只留占位，记一行 `WARN forward not expanded venue=… message=… why=…`；这一条里后面要调的不再调（同 @ 的名字，「施工时
//!   定的」第 59 条：跟核心的那一头一条条照先后办，NapCat 一卡，一条四个转发就要等四个时限）。
//! - 每条照同一套认段（`segments`）：字照原样接，@ 写成 `@名字`（群里照群成员的缓存，不另问；没有的、私聊的写号），带的东西
//!   同 `venue.media` 的洗法；名字照节点的发的人（群名片、昵称，洗法同 `name`），都没有的写号，号也没有的这一条不放、算进
//!   `more`。里面又是 `forward` 的照层数往下取。
//! - 限额（`bridge.json`）：一条消息（转发里的一条也算一条）最多展开 `message_forwards` 个转发，多的只留占位；套
//!   `forward_depth` 层，再深的只留占位；一个转发最多放 `forward_nodes` 条，多的记进 `more`；每条正文截到 `forward_chars` 个字符，
//!   再夹到核心的 4096 字节。
//!   另照核心的整条上限（所有转发合起来 100 条、名字和正文 16 KiB）一条条放，放不下的算进 `more`：超了核心整条拒，这条消息就丢了。
//!   一条都放不进的转发不交。
//! - 带的东西的编号（施工 I-75，`crate::media::numbered` 那一套）：接着这条消息自己带的数，放进去的那几样带上 `index`，核心写成
//!   `[image #<编号>: …]`，她照这条消息的 `msg` 和编号 `fetch_media`。没放进去的几条照样数；打不开的转发（取不到的，照限额没去取、
//!   又没内联的）到这里为止，后面的不带编号。

use std::sync::Arc;
use std::time::Instant;

use futures_util::future::BoxFuture;
use serde_json::{Value, json};

use super::fields::{NAME, clean, fits, media};
use super::{Message, Route};
use crate::TARGET;
use crate::chat::{CallError, FORWARD, Forwarded, Id, Node, Piece, Segments, To};
use crate::chat::{Link, Platform};
use crate::core::Gone;
use crate::toolset::{Numbering, kept};

/// 核心的整条上限（`venues.md`「场所的格」的 `forwards`）：一条场所消息里所有转发合起来最多几条。
const ALL_NODES: usize = 100;
/// 同上：所有转发的名字和正文合起来最多几个字节。
const ALL_BYTES: usize = 16 * 1024;
/// 同上：转发里一条的正文最多几个字节。
const TEXT_BYTES: usize = 4096;

/// 展开合并转发的几个数（`bridge.json`，施工 O-34）。
#[derive(Debug, Clone, Copy)]
pub(crate) struct Forwarding {
    /// 一条消息最多展开几个转发（`message_forwards`）。
    pub(crate) per_message: usize,
    /// 最多套几层（`forward_depth`）：消息里的转发是第一层。
    pub(crate) depth: usize,
    /// 一个转发最多放几条（`forward_nodes`）。
    pub(crate) nodes: usize,
    /// 每条正文最多几个字符（`forward_chars`）。
    pub(crate) chars: usize,
}

/// 要展开的一条消息：收进它的机器人号、在哪（群、私聊的对方）、平台编号（只记运行日志）、自己带了几样、它的转发、它引用的
/// 那一条（施工 I-238，`quoted`；私聊的施工 I-245）。
pub(super) struct Forwards {
    /// 收进它的机器人号：经它的连接取。
    pub(super) bot: Id,
    /// 在哪：群里的 @ 的名字照这个群的缓存；引用的那一条照这里取。
    pub(super) to: To,
    /// 平台编号。
    pub(super) msg: i64,
    /// 自己带了几样（`venue.media` 那几样，施工 I-75）：转发里的接着它编号。
    pub(super) own: usize,
    /// 转发，照段的先后。
    pub(super) forwards: Vec<Forwarded>,
    /// 引用的那一条的平台编号（`quoted`）：取回来写进场所的格的 `quoted`。
    pub(super) quoted: Option<String>,
}

impl Forwards {
    /// 机器人号 `bot` 收进来的、在 `to`（群、私聊的对方）、平台编号是 `msg` 的那一条（认出来的段 `segments`）：引用了一条的
    /// 另取那一条（施工 I-238，私聊的 I-245）。没有转发、没有引用的是空的。
    pub(super) fn of(bot: Id, to: To, msg: i64, segments: &Segments) -> Option<Forwards> {
        let quoted = segments.reply_to.clone().filter(|id| fits(id));
        (!segments.forwards.is_empty() || quoted.is_some()).then(|| Forwards {
            bot,
            to,
            msg,
            own: kept(&segments.media).len(),
            forwards: segments.forwards.clone(),
            quoted,
        })
    }
}

/// 这一条消息还放得下多少（核心的整条上限）、还调不调 `get_forward_msg`、带的东西编到哪了（施工 I-75）。
pub(super) struct Budget {
    nodes: usize,
    bytes: usize,
    asking: bool,
    pub(super) numbers: Numbering,
}

/// 取的那一头：连接和收进这条的机器人号、平台、在哪（群、私聊的对方）、记运行日志的场所和编号。
pub(super) struct At<'a> {
    pub(super) link: Option<Arc<dyn Link>>,
    pub(super) bot: Id,
    platform: Platform,
    pub(super) to: To,
    pub(super) venue: &'a str,
    pub(super) msg: i64,
}

impl Route {
    /// 交 `message` 以前展开它的转发，写进场所的格的 `forwards`，再取它引用的那一条写进 `quoted`（施工 I-238，[`Route::quoted`]：
    /// 放在转发后面，整条上限先给转发；都没有的什么都不做）：先找到会话，不接的不去取，交回假，这条不送。
    ///
    /// # Errors
    ///
    /// 找会话时核心断开。
    pub(super) async fn expanded(&mut self, message: &mut Message) -> Result<bool, Gone> {
        let Some(wanted) = message.forwards.as_ref() else {
            return Ok(true);
        };
        if !self.found(&message.place).await? {
            return Ok(false);
        }
        let venue = message.place.peer.venue.to_string();
        let (at, mut budget) = self.start((message.place.peer.platform, wanted), &venue);
        let forwards = self.expand(&at, &wanted.forwards, 1, &mut budget).await;
        if !forwards.is_empty() {
            message.fields["forwards"] = Value::Array(forwards);
        }
        if let Some(quoted) = &wanted.quoted
            && let Some(one) = self.quoted(&at, quoted, &mut budget).await
        {
            message.fields["quoted"] = one;
        }
        Ok(true)
    }

    /// 取 `wanted` 的那一头和整条的上限：经平台 `platform` 上收进它的号取。`venue` 只记运行日志。
    fn start<'a>(
        &self,
        (platform, wanted): (Platform, &Forwards),
        venue: &'a str,
    ) -> (At<'a>, Budget) {
        let at = At {
            link: self.platforms.link(platform, &wanted.bot),
            bot: wanted.bot.clone(),
            platform,
            to: wanted.to.clone(),
            venue,
            msg: wanted.msg,
        };
        let budget = Budget {
            nodes: ALL_NODES,
            bytes: ALL_BYTES,
            asking: true,
            numbers: Numbering::after(wanted.own),
        };
        (at, budget)
    }

    /// 第 `depth` 层的几个转发：照限额和 `budget` 放。不去取的、没放进去的照样编号（[`Numbering`]）。
    fn expand<'a>(
        &'a self,
        at: &'a At<'a>,
        forwards: &'a [Forwarded],
        depth: usize,
        budget: &'a mut Budget,
    ) -> BoxFuture<'a, Vec<Value>> {
        Box::pin(async move {
            let limits = self.forwarding;
            let mut out = Vec::new();
            if depth > limits.depth {
                budget.numbers.skip(forwards);
                return out;
            }
            for (taken, forwarded) in forwards.iter().enumerate() {
                if taken >= limits.per_message {
                    budget.numbers.skip(std::slice::from_ref(forwarded));
                    continue;
                }
                let Some(messages) = messages(at, forwarded, budget).await else {
                    budget.numbers.stop();
                    continue;
                };
                let mut nodes = Vec::new();
                let mut placing = true;
                for (seen, read) in messages.iter().enumerate() {
                    placing &= seen < limits.nodes && budget.nodes > 0;
                    let placed = match placing {
                        true => self.node(at, read, depth, budget).await,
                        false => Err(false),
                    };
                    match placed {
                        Ok(one) => nodes.push(one),
                        Err(full) => {
                            placing &= !full;
                            budget.numbers.count(std::slice::from_ref(read));
                        }
                    }
                }
                if nodes.is_empty() {
                    continue;
                }
                let more = messages.len() - nodes.len();
                let mut forward = json!({"nodes": nodes});
                if more > 0 {
                    forward["more"] = json!(more);
                }
                out.push(forward);
            }
            out
        })
    }

    /// 放转发里的一条 `read`（第 `depth` 层；引用的那一条是第 0 层，它里面的转发是第一层）：带的东西编上号，套着的转发往下
    /// 展开。没名字的交回 `Err(false)`，放不下的 `Err(true)`（这个转发后面的不再放）：都不动 `budget`，调的一方照只数不放的办。
    pub(super) async fn node(
        &self,
        at: &At<'_>,
        read: &Node,
        depth: usize,
        budget: &mut Budget,
    ) -> Result<Value, bool> {
        let name = read
            .name
            .as_deref()
            .and_then(|name| clean(name, NAME))
            .or_else(|| read.user.clone().map(|user| user.to_string()))
            .ok_or(false)?;
        let text = clip(&self.words(at, &read.segments), self.forwarding.chars);
        let bytes = name.len() + text.len();
        if bytes > budget.bytes {
            return Err(true);
        }
        budget.nodes -= 1;
        budget.bytes -= bytes;
        let mut one = json!({"name": name, "text": text});
        let mut carried = media(&read.segments.media);
        for item in &mut carried {
            if let Some(number) = budget.numbers.take() {
                item["index"] = json!(number);
            }
        }
        if !carried.is_empty() {
            one["media"] = Value::Array(carried);
        }
        if !read.segments.forwards.is_empty() {
            let inner = self
                .expand(at, &read.segments.forwards, depth + 1, budget)
                .await;
            if !inner.is_empty() {
                one["forwards"] = Value::Array(inner);
            }
        }
        Ok(one)
    }

    /// 转发里一条的正文：段照先后接，@ 写成 `@名字`（群里照 `at` 那个平台、那个群的成员缓存，不另问 NapCat；没有的、私聊的写号）。
    fn words(&self, at: &At<'_>, segments: &Segments) -> String {
        let mut text = String::new();
        for piece in &segments.pieces {
            match piece {
                Piece::Text(words) => text.push_str(words),
                Piece::At(user) => {
                    text.push('@');
                    let group = match &at.to {
                        To::Group(group) => Some(group.clone()),
                        To::Private(_) => None,
                    };
                    let name = group
                        .and_then(|group| {
                            self.members
                                .name((at.platform, group, user.clone()), Instant::now())
                        })
                        .and_then(|name| clean(name, NAME));
                    match name {
                        Some(name) => text.push_str(&name),
                        None => text.push_str(&user.to_string()),
                    }
                }
            }
        }
        text
    }
}

/// 一个转发里的消息：内联的照它；没有的、`budget` 还让问的调 `get_forward_msg`。取不到的记一行运行日志、交回空的，调不成的
/// 以后不再调。
async fn messages(at: &At<'_>, forwarded: &Forwarded, budget: &mut Budget) -> Option<Vec<Node>> {
    if let Some(content) = &forwarded.content {
        return Some(content.clone());
    }
    let why = match (&forwarded.id, &at.link) {
        (None, _) => "no-id",
        (Some(_), _) if !budget.asking => "skipped",
        (Some(_), None) => "unreachable",
        (Some(id), Some(link)) => match link.forwarded(id).await {
            Ok(Some(messages)) => return Some(messages),
            Ok(None) => "unreadable",
            Err(CallError::Failed { .. }) => "failed",
            Err(CallError::Timeout) => "unanswered",
            Err(CallError::Closed) => "unreachable",
            Err(CallError::Unsupported) => "unsupported",
        },
    };
    if why != "no-id" {
        budget.asking = false;
    }
    tracing::warn!(target: TARGET, venue = at.venue, message = at.msg, why, "forward not expanded");
    None
}

/// 私聊的正文（施工 O-34）：只取 `text` 段的字（第 6 条）后面，每个合并转发空一格接一个占位，同群消息正文里的写法；O-34
/// 以前造的会话只看得到占位。QQ 里的合并转发是单独的一条，不和字混着发，接在后面不错位。
pub(super) fn with_placeholders(text: &str, forwards: usize) -> String {
    let mut out = text.to_string();
    for _ in 0..forwards {
        if !out.trim().is_empty() {
            out.push(' ');
        }
        out.push_str(FORWARD);
    }
    out
}

/// 转发里一条的正文：截到 `chars` 个字符，再夹到核心的 [`TEXT_BYTES`] 字节（截在字的边界上）。`forward_chars` 调大了、全是
/// 4 字节的字（1025 个以上）也不超核心的上限：超了核心整条拒，这条消息就丢了（2026-10-11 核心审时建议）。
pub(super) fn clip(text: &str, chars: usize) -> String {
    let mut out = String::new();
    for c in text.chars().take(chars) {
        if out.len() + c.len_utf8() > TEXT_BYTES {
            break;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests;
