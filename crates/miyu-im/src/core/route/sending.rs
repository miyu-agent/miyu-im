//! 出站队列的那一头（施工 O-25 中，`onebot.md` 第一条「出站队列」；18 第十节、Q15）：她要说出去的一切（群里她的话、限流的
//! 提示、命令回执，私聊的回话）先记 `ext.im.venues.queued`、拿到序号，再照先后交 NapCat；结局记下：群里她的话记
//! `venue.delivered`，群里的回执回了编号的交 `receipt` 撤，没成、过期的记 `ext.im.venues.failed`。
//!
//! - 门开着（这个群没被禁言，照投影；收进这个会话的那个机器人号连着）的照先后放进那个号现在的连接的写队列，前一段放进去了才
//!   放下一段；等回应交给别的任务，结局照放进写队列的先后交回来（`Route` 的 `sending`，「施工时定的」第 10、84 条）。
//! - 门关着的排着（纯逻辑在 `queue`）。什么时候再看一遍：入队以后、机器人号连上了、记了禁言或解禁以后（`muted`）、定时醒了
//!   （[`Route::wake`]，跟核心的那一头睡到那一刻）：先把过期的记了，再交门开着的（「施工时定的」第 123、125 条）。
//! - 她的话入队记成了就算进这一回合发出去的（去重照它）：群里先算进投影，私聊记进 `Spoken`（「施工时定的」第 112、113 条）。
//! - 退信（施工 O-25 下，「退信」）：她的话记了 `failed` 以后，经核心的 `session.note` 给那个会话记一块 `undelivered` 事实，写着
//!   为什么和那一段的开头（模板是出厂数据，`rules/facts.rs`）；一段一块，提示、回执不退（「施工时定的」第 129、130 条）。在 `bounce`。

use std::sync::Arc;
use std::time::Duration;

use miyu_client::ids::ExternalId;
use miyu_client::time::Timestamp;
use serde_json::{Value, json};

use super::outbound::Aimed;
use super::projection::{QUEUED, REPLY};
use super::queue::{DISCONNECTED, EXPIRED, Ending, ending, said};
use super::receipt::recall;
use super::{Peer, Route, applied};
use crate::TARGET;
use crate::chat::{CallError, Lead, Outgoing, To};
use crate::core::Gone;
use crate::counters::Count;

mod aiming;
mod bounce;

/// 失败记成的事件（第 4 条）。
const FAILED: &str = "ext.im.venues.failed";

/// 发出去了记成的事件（`venues.md`「桥记的事件」第 3 条）。
const DELIVERED: &str = "venue.delivered";

/// `ext.im.venues.queued` 的 `kind`：命令回执、被拒的那一句。
const RECEIPT: &str = "receipt";

/// `ext.im.venues.queued` 的 `kind`：她发出去的一张图（施工 O-51）。
const IMAGE: &str = "image";

/// 门关着：她被禁言着。
const MUTED: &str = "muted";

/// 退信那一块事实的 `kind`（施工 O-25 下，「退信」第 2 条）。
const UNDELIVERED: &str = "undelivered";

/// 退信带她那一段的头几个字符（施工 O-25 下，「退信」第 3 条，「施工时定的」第 130 条）：认得出是哪一句就够。
const HEAD: usize = 30;

/// 要说出去的一段是什么（第 1 条）。
#[derive(Debug, Clone)]
pub(super) enum What {
    /// 她的话：哪条线、回合编号、这一轮回的人（群里的成了记 `venue.delivered`）。
    Reply {
        /// 她在哪条线说的：那条线的会话编号（主线、私聊是这个会话自己；支线是子会话，施工 O-36）。
        line: String,
        /// 回合编号（那条线的）。
        turn: u64,
        /// 这一轮回的人；私聊的是空的。
        to: Vec<ExternalId>,
    },
    /// 她发出去的一张图（施工 O-51，`stickers`：核心的工具结果里的 `sent`，表情包包发的那种）：一张一条，正文是空的。
    Image(Box<Image>),
    /// 她的一句长文画成的一页（施工 O-59 上，`long`）：第一页的正文是全文，算她的话；后面几页正文是空的。
    Page(Box<Page>),
    /// 命令回执、被拒的那一句：`recall` 是群里回了编号的要不要过几秒撤回（施工 O-47：照 `im.recall_receipts`）。
    Receipt {
        /// 撤不撤。
        recall: bool,
    },
}

/// 她发出去的一张图（施工 O-51）。
#[derive(Debug, Clone)]
pub(super) struct Image {
    /// 工具结果出现在的那条线（主线、支线、私聊）：发不出去的 `unshown` 记进它。
    pub(super) line: String,
    /// 那条线的回合编号。
    pub(super) turn: u64,
    /// 图的内容哈希（blob 的编号）：群里发出去了记进 `venue.delivered` 的 `images`。
    pub(super) blob: String,
    /// 名字：发不出去的 `unshown` 写它。
    pub(super) name: String,
    /// 是表情包：QQ 当表情显示（`sub_type=1`）。
    pub(super) sticker: bool,
    /// NapCat 认的 `base64://…`：入队以前读好。
    pub(super) file: String,
}

/// 长文画成的一页（施工 O-59 上）：图照发图的写法（线、回合、blob、名字、读好的 `base64://…`），另带她回的人、是不是第一页。
#[derive(Debug, Clone)]
pub(super) struct Page {
    /// 这一页的图。
    pub(super) image: Image,
    /// 这一轮回的人；私聊的是空的。只有第一页记进 `venue.delivered`。
    pub(super) to: Vec<ExternalId>,
    /// 是第一页：带全文、引用和 @，算她的话（进投影、退信）。
    pub(super) first: bool,
}

/// 要说出去的一段：是什么、正文（不带引用、@）、前面带的引用和 @、正文发成什么样。
#[derive(Debug, Clone)]
pub(super) struct Piece {
    /// 是什么。
    pub(super) what: What,
    /// 正文。
    pub(super) text: String,
    /// 前面带的引用和 @：只有她一句话的第一段带。
    pub(super) lead: Lead,
    /// 正文发成什么样（施工 O-59 下）：字、图、合并转发。
    pub(super) body: Body,
    /// 离同一个会话前一段过多久才发（施工 I-26，模拟真人一条分多条发送）：只有她一句话拆出来的第二段起有，别的是零。
    pub(super) pause: Duration,
    /// 群里她一句话的第一段：本来想要的引用、@（施工 I-64），交给 NapCat 的那一刻照那时群里的样子再定 `lead`；别的是空的。
    pub(super) aimed: Option<Aimed>,
}

/// 正文发成什么样（施工 O-59 下，`onebot.md`「长文」）：她发出去的图、长文画成的一页照旧在 [`What`] 里带着图。
#[derive(Debug, Clone, Default)]
pub(super) enum Body {
    /// 一段字（`text`）。
    #[default]
    Text,
    /// 一张图：NapCat 认的 `base64://…`（回执画成的长图）。
    Picture(String),
    /// 一条合并转发：几段字一个节点一段，发的人的名字。
    Forward {
        /// 节点的字，照先后。
        nodes: Vec<String>,
        /// 节点上写的名字。
        nickname: String,
    },
}

/// 排着的一段：入队那一条的序号和这一段。
#[derive(Debug)]
pub(super) struct Item {
    /// 哪个会话要说的（主线、支线）：队照场所排（施工 I-58），记、发照它。
    session: String,
    /// 入队那一条（`ext.im.venues.queued`）的序号：`failed` 的 `queued` 照它写。
    queued: u64,
    /// 这一段。
    piece: Piece,
}

/// 交出去的一段等来了回应：哪个会话、发到哪、这一段、调用的结果。
#[derive(Debug)]
pub(super) struct Answered {
    /// 会话编号。
    session: String,
    /// 发到哪：交出去那一刻的。
    peer: Peer,
    /// 这一段。
    item: Item,
    /// 调用的结果。
    result: Result<Option<i64>, CallError>,
}

impl Route {
    /// 会话 `session` 要说一段 `piece`（第 2 条）：先记 `ext.im.venues.queued`（命令编号是 `id`，空的自己编），记成了排进
    /// 队、算进这一回合发出去的（她的话），再看一遍排着的。去掉首尾空白是空的不入队（第 1 条）；不知道发到哪的、本机的钟读不出的
    /// （照说不会）、核心拒了的（照说不会，`append` 记了一行）不发。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn enqueue(
        &mut self,
        session: &str,
        id: Option<&str>,
        piece: Piece,
    ) -> Result<(), Gone> {
        let Some(peer) = self.peers.get(session).cloned() else {
            return Ok(());
        };
        let bodied = !matches!(piece.body, Body::Text);
        if piece.text.trim().is_empty()
            && !bodied
            && !matches!(piece.what, What::Image(_) | What::Page(_))
        {
            return Ok(());
        }
        let chars = piece.text.chars().count();
        let Some(clock) = applied::clock() else {
            tracing::warn!(target: TARGET, venue = %peer.venue, chars, "clock not readable, reply dropped");
            return Ok(());
        };
        let body = queued_body(&piece);
        let Some(queued) = self.append(session, id, QUEUED, body).await? else {
            return Ok(());
        };
        let said = match &piece.what {
            What::Reply { line, turn, .. } => Some((line, *turn)),
            What::Page(page) if page.first => Some((&page.image.line, page.image.turn)),
            _ => None,
        };
        if let Some((line, turn)) = said {
            match (peer.to, self.groups.get_mut(session)) {
                (To::Group(_), Some(group)) => group.queued(line, turn, &piece.text),
                (To::Group(_), None) => {}
                (To::Private(_), _) => self
                    .spoken
                    .entry(session.to_string())
                    .or_default()
                    .add(turn, piece.text.clone()),
            }
        }
        if let Some(why) = self.shut(session, clock.now) {
            tracing::info!(target: TARGET, venue = %peer.venue, why, chars, "reply waiting");
        }
        let pause = piece.pause;
        let item = Item {
            session: session.to_string(),
            queued,
            piece,
        };
        self.waiting
            .push(peer.venue.as_str(), (clock.now, pause), item);
        self.pump().await
    }

    /// 把有东西排着的场所都看一遍（第 3 条）：过了期的记 `expired`，门开着的照先后交 NapCat，一个场所一次一段（施工 I-48、I-58）：
    /// 交出去的回了才交下一段（[`Route::answered`] 再看一遍）；当场就交不成的照结局办，接着交下一段。门开没开照排在最前的那一段的
    /// 会话看。本机的钟读不出的（照说不会）记一行，这一回不看。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn pump(&mut self) -> Result<(), Gone> {
        let Some(clock) = applied::clock() else {
            tracing::warn!(target: TARGET, "clock not readable, queue not looked at");
            return Ok(());
        };
        for line in self.waiting.lines() {
            loop {
                let open = self
                    .waiting
                    .front(&line)
                    .is_some_and(|item| self.shut(&item.session, clock.now).is_none());
                let taken = self.waiting.take(&line, clock.now, open);
                for item in taken.expired {
                    self.failed(&item.session.clone(), &item, (EXPIRED, None))
                        .await?;
                }
                let Some(item) = taken.ready.into_iter().next() else {
                    break;
                };
                if self.hand(&line, item).await? {
                    break;
                }
            }
        }
        Ok(())
    }

    /// 离下一次该看一遍还有多久（第 8 条）：排着的里最早的过期时刻、被禁言着的会话禁言到期的时刻；没有排着的是空的，不用醒。
    pub(super) fn wake(&self) -> Option<Duration> {
        let now = applied::clock()?.now;
        let queued = self.waiting.wake(now, |venue| {
            let session = self.venues.get(venue)?;
            self.groups.get(session)?.muted(now)
        });
        // 还没记解禁的禁言到点也醒（施工 O-50，`muted`）。
        let at = queued.into_iter().chain(self.mute_wake()).min()?;
        let millis = at.unix_millis().saturating_sub(now.unix_millis());
        Some(Duration::from_millis(u64::try_from(millis).unwrap_or(0)))
    }

    /// 会话 `session` 此刻（`now`）门关着的为什么：这个群被禁言着是 `muted`，收进它的那个机器人号没连着是 `disconnected`；
    /// 开着的是空的。
    fn shut(&self, session: &str, now: Timestamp) -> Option<&'static str> {
        if let Some(group) = self.groups.get(session)
            && group.muted(now).is_some()
        {
            return Some(MUTED);
        }
        let connected = self
            .peers
            .get(session)
            .is_some_and(|peer| self.bots.get(peer.bot).is_some());
        (!connected).then_some(DISCONNECTED)
    }

    /// 把场所 `line` 排着的一段交 NapCat：放进收进它那个会话的那个机器人号现在的连接的写队列，等回应交给别的任务（第 3 条），
    /// 交回真：这个场所等它回了再交下一段。放不进去的（那个号这时没连着、连接正在断）当场照结局办，交回假：接着交下一段。
    async fn hand(&mut self, line: &str, mut item: Item) -> Result<bool, Gone> {
        let session = item.session.clone();
        let Some(peer) = self.peers.get(&session).cloned() else {
            self.waiting.answered(line);
            self.failed(&session, &item, (DISCONNECTED, None)).await?;
            return Ok(false);
        };
        // 群里一句话的第一段：照这一刻群里的样子再定带不带引用、@，记下她交出去的这一刻（施工 I-64）。
        let group = self.venues.get(peer.venue.as_str()).cloned();
        if let Some(aimed) = item.piece.aimed.take() {
            self.relead_now(&peer, group.as_deref(), &aimed, &mut item.piece.lead);
        }
        let begun = match self.bots.get(peer.bot) {
            Some(link) => {
                let lead = &item.piece.lead;
                let outgoing = match (&item.piece.body, &item.piece.what) {
                    (Body::Picture(file), _) => Outgoing::Picture { file, lead },
                    (Body::Forward { nodes, nickname }, _) => Outgoing::Forward {
                        bot: peer.bot,
                        nickname,
                        nodes,
                    },
                    (Body::Text, What::Image(image)) => Outgoing::Image {
                        file: &image.file,
                        sticker: image.sticker,
                    },
                    (Body::Text, What::Page(page)) => Outgoing::Picture {
                        file: &page.image.file,
                        lead,
                    },
                    (Body::Text, _) => Outgoing::Text {
                        text: &item.piece.text,
                        lead,
                    },
                };
                link.begin(peer.to, outgoing).await
            }
            None => Err(CallError::Closed),
        };
        match begun {
            Ok(pending) => {
                if let Some(projection) =
                    group.as_deref().and_then(|main| self.groups.get_mut(main))
                {
                    projection.handed();
                }
                self.sending.push_back(tokio::spawn(async move {
                    let result = pending.await;
                    Answered {
                        session,
                        peer,
                        item,
                        result,
                    }
                }));
                Ok(true)
            }
            Err(error) => {
                self.waiting.answered(line);
                let result = Err(error);
                self.ended(Answered {
                    session,
                    peer,
                    item,
                    result,
                })
                .await?;
                Ok(false)
            }
        }
    }

    /// 交出去的一段 NapCat 回了（第 4 条）：照结局办，这个会话的下一段可以交了，再看一遍队（施工 I-48）。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn answered(&mut self, answered: Answered) -> Result<(), Gone> {
        self.waiting.answered(answered.peer.venue.as_str());
        self.ended(answered).await?;
        self.pump().await
    }

    /// 交出去的一段有了结局（第 4 条）：没成的记 `failed`；成了的记一行，群里她的话记 `venue.delivered`（回了编号的），群里的回执
    /// 回了编号的过几秒撤回（「斜杠命令」第 7 条），别的不再记。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    async fn ended(&mut self, answered: Answered) -> Result<(), Gone> {
        let Answered {
            session,
            peer,
            item,
            result,
        } = answered;
        let msg = match ending(&result) {
            Ending::Sent(msg) => msg,
            Ending::Failed(why, detail) => {
                return self.failed(&session, &item, (why, detail)).await;
            }
        };
        self.bounced.remove(&session);
        let (venue, chars) = (&peer.venue, item.piece.text.chars().count());
        let text = crate::recent::Note::excerpt(&item.piece.text);
        tracing::info!(target: TARGET, venue = %venue, chars, text = %text, "reply sent");
        // 聊天记录（设计 04，施工 C-1 上）：她发出去了的照记（回了编号的）。
        if let Some(msg) = msg {
            let image = matches!(item.piece.what, What::Image(_) | What::Page(_));
            self.record_mine(&venue.to_string(), peer.bot, (msg, &item.piece.text), image);
        }
        if said(&item.piece.what) {
            self.counters.bump(Count::Replied, &venue.to_string());
        }
        if !matches!(peer.to, To::Group(_)) {
            return Ok(());
        }
        match (item.piece.what, msg) {
            (What::Reply { line, turn, to }, Some(msg)) => {
                let body = json!({"line": line, "turn": turn, "to": to, "msg": msg.to_string(), "text": item.piece.text, "images": []});
                self.append(&session, None, DELIVERED, body).await.map(drop)
            }
            // 长文的一页（施工 O-59 上）：第一页同她的话（全文、回的人），后面几页只有图；`images` 都是那一页。
            (What::Page(page), Some(msg)) => {
                let to = if page.first { page.to } else { Vec::new() };
                let body = json!({"line": page.image.line, "turn": page.image.turn, "to": to, "msg": msg.to_string(), "text": item.piece.text, "images": [page.image.blob]});
                self.append(&session, None, DELIVERED, body).await.map(drop)
            }
            (What::Reply { .. } | What::Image(_) | What::Page(_), None) => {
                tracing::warn!(target: TARGET, venue = %venue, chars, "delivered without a message id");
                Ok(())
            }
            // 她发出去的图（施工 O-51）：正文是空的，`images` 是它。回的人不算（图不引用、不 @）。
            (What::Image(image), Some(msg)) => {
                let body = json!({"line": image.line, "turn": image.turn, "to": [], "msg": msg.to_string(), "text": "", "images": [image.blob]});
                self.append(&session, None, DELIVERED, body).await.map(drop)
            }
            // 不撤的回执（施工 O-47，`im.recall_receipts` 里没有的命令）发了就完。
            (What::Receipt { recall: false }, _) => Ok(()),
            (What::Receipt { recall: true }, Some(msg)) => {
                let bots = Arc::clone(&self.bots);
                self.chores.spawn(recall(msg, peer, bots, self.recall));
                Ok(())
            }
            (What::Receipt { recall: true }, None) => {
                tracing::warn!(target: TARGET, venue = %venue, "receipt not recalled, no message id");
                Ok(())
            }
        }
    }
}

/// 入队那一条的 `body`（第 2 条）：`{kind, text}`，她的话（长文的第一页也是，另带 `blob`）另带 `line`（她说它的那条线，主线或者支线，施工 O-36）、`turn`，提示
/// 另带 `reason`。
fn queued_body(piece: &Piece) -> Value {
    match &piece.what {
        What::Reply { line, turn, .. } => {
            let mut body = json!({"kind": REPLY, "text": piece.text, "line": line, "turn": turn});
            // 打成合并转发的长文（施工 O-59 下）。
            if matches!(piece.body, Body::Forward { .. }) {
                body["forward"] = json!(true);
            }
            body
        }
        What::Receipt { .. } => json!({"kind": RECEIPT, "text": piece.text}),
        What::Image(image) => {
            json!({"kind": IMAGE, "blob": image.blob, "sticker": image.sticker, "line": image.line, "turn": image.turn})
        }
        // 长文的第一页记成她的话（去重、补发都认它），另带 blob；后面几页记成图（施工 O-59 上）。
        What::Page(page) if page.first => {
            json!({"kind": REPLY, "text": piece.text, "line": page.image.line, "turn": page.image.turn, "blob": page.image.blob})
        }
        What::Page(page) => {
            json!({"kind": IMAGE, "blob": page.image.blob, "sticker": false, "line": page.image.line, "turn": page.image.turn})
        }
    }
}
