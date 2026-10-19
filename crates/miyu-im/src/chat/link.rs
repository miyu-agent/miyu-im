//! 一个机器人号的一条连接（施工 I-2 上，设计 01 第三节）：宿主只经它碰平台。要做的事是宿主的说法（发一段字、撤回、禁言、
//! 群成员……），驱动照自己的平台拼调用、读回应，交回宿主认的样子；平台的动作名、回应原文不进宿主。
//!
//! 异步的方法交回装箱的 future（[`Reply`]）：接口是 trait 对象（`Arc<dyn Link>`），施工单「施工时定的」第 1 条。

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use serde_json::Value;

use super::{CallError, Fetch, Lead, Node, Rank, Segments, Source, To};

/// 一次调用的结局。
pub type Reply<'a, T> = Pin<Box<dyn Future<Output = Result<T, CallError>> + Send + 'a>>;

/// 放进了写队列、在等回应的一次发送（[`Link::begin`]）：等到的是发出去那一条的平台编号，平台没回编号的是空的。
pub type Pending = Pin<Box<dyn Future<Output = Result<Option<i64>, CallError>> + Send>>;

/// 发出去的一样。
#[derive(Debug, Clone, Copy)]
pub enum Outgoing<'a> {
    /// 一句字，前面带 `lead` 要带的引用和 @。
    Text {
        /// 字。
        text: &'a str,
        /// 带的引用和 @。
        lead: &'a Lead,
    },
    /// 一张图（`file` 写成 `base64://…`）；`sticker` 的当表情包发（施工 O-51）。
    Image {
        /// 图。
        file: &'a str,
        /// 当表情包发。
        sticker: bool,
    },
    /// 长文画成的一页（施工 O-59 上）：先是 `lead` 要带的引用和 @，再是图，同一条消息。
    Picture {
        /// 图。
        file: &'a str,
        /// 带的引用和 @。
        lead: &'a Lead,
    },
    /// 一条合并转发（施工 O-59 下）：每一段 `nodes` 一个节点，发的人照机器人号 `bot`、名字 `nickname`。
    Forward {
        /// 机器人号。
        bot: i64,
        /// 节点上写的名字。
        nickname: &'a str,
        /// 每个节点的字。
        nodes: &'a [String],
    },
    /// 一份文件（`file` 写成 `base64://…`，文件名 `name`，施工 O-46）。
    Upload {
        /// 文件。
        file: &'a str,
        /// 文件名。
        name: &'a str,
    },
}

/// 平台上一条消息的编号（[`Link::recall`]）：平台回的是整数（回执，施工 O-25 上）；她给的、引用里的照原样的字还回去（施工
/// O-31），驱动别换写法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg<'a> {
    /// 平台回的整数编号。
    Number(i64),
    /// 照原样的字。
    Text(&'a str),
}

/// 取回来的一条消息（[`Link::message`]）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fetched {
    /// 是私聊里的。
    pub private: bool,
    /// 群里的：群号；读不出的是空的。
    pub group: Option<i64>,
    /// 发的人的号；读不出的是空的。
    pub user: Option<i64>,
    /// 认出来的段。
    pub segments: Segments,
}

/// 平台说要取的那一样在哪（[`Link::fetch`]）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Located {
    /// 在哪，照先后试。
    pub sources: Vec<Source>,
    /// 平台说的文件名，去掉首尾空白不空的才有。
    pub name: Option<String>,
}

/// 一个群成员（[`Link::member`]、[`Link::members`]）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// 号。
    pub user: i64,
    /// 在群里叫什么：群名片，空白的取昵称；都没有的是空的。照原样，没洗。
    pub name: Option<String>,
    /// 群里的身份；没带、认不出的是空的。
    pub rank: Option<Rank>,
}

/// 机器人在的一个群（[`Link::groups`]）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// 群号。
    pub id: i64,
    /// 群名，照原样；没有的是空的。
    pub name: Option<String>,
    /// 多少人；没有的是空的。
    pub members: Option<u64>,
}

/// 机器人的一个好友（[`Link::friends`]）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Friend {
    /// 号。
    pub id: i64,
    /// 昵称，照原样；没有的是空的。
    pub name: Option<String>,
}

/// 对端是哪个实现（施工 O-28：后台页的 `status`、状态文件说它）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Implementation {
    /// 实现的名字，例如 `NapCat.Onebot`。
    pub name: String,
    /// 实现的版本。
    pub version: String,
}

/// 一个机器人号的一条连接。
pub trait Link: Send + Sync + fmt::Debug {
    /// 把要发的放进写队列（照调的先后，出站队列要它），交回等回应的那一段。
    ///
    /// # Errors
    ///
    /// 连接已经断了：[`CallError::Closed`]。
    fn begin<'a>(&'a self, to: To, outgoing: Outgoing<'a>) -> Reply<'a, Pending>;

    /// 发出去，等到回应：发出去那一条的平台编号，没回编号的是空的。
    ///
    /// # Errors
    ///
    /// 同 [`Link::begin`]；等的时候断了、过了时、平台回了没成。
    fn send<'a>(&'a self, to: To, outgoing: Outgoing<'a>) -> Reply<'a, Option<i64>> {
        Box::pin(async move { self.begin(to, outgoing).await?.await })
    }

    /// 撤回编号是 `message` 的那一条（施工 O-25 上、O-31）。
    fn recall<'a>(&'a self, message: Msg<'a>) -> Reply<'a, ()>;

    /// 在群 `group` 里禁言号是 `user` 的人 `seconds` 秒，0 是解禁（施工 O-31）。
    fn mute(&self, group: i64, user: i64, seconds: u64) -> Reply<'_, ()>;

    /// 在 `to` 那里戳一戳号是 `user` 的人（施工 O-31）。
    fn poke(&self, to: To, user: i64) -> Reply<'_, ()>;

    /// 在平台编号是 `message` 的那一条上贴（`on`）、摘表情 `emoji`（施工 O-25 下）。
    fn react<'a>(&'a self, message: &'a str, emoji: &'a str, on: bool) -> Reply<'a, ()>;

    /// 同意标记是 `flag` 的那次好友请求（施工 O-27）。
    fn befriend<'a>(&'a self, flag: &'a str) -> Reply<'a, ()>;

    /// 取平台编号是 `id` 的那一条消息（施工 O-33）。
    fn message<'a>(&'a self, id: &'a str) -> Reply<'a, Fetched>;

    /// 问平台照 `fetch` 取编号是 `id` 的那一样在哪，最多等 `wait`（施工 O-33：平台要先把东西下下来，等得比别的久）。
    fn fetch<'a>(&'a self, fetch: Fetch, id: &'a str, wait: Duration) -> Reply<'a, Located>;

    /// 取编号是 `id` 的合并转发里的消息（施工 O-34）：回了但读不出的是空的。
    fn forwarded<'a>(&'a self, id: &'a str) -> Reply<'a, Option<Vec<Node>>>;

    /// 群 `group` 的成员（施工 O-54）。
    fn members(&self, group: i64) -> Reply<'_, Vec<Member>>;

    /// 群 `group` 里号是 `user` 的那个人（施工 O-22）。
    fn member(&self, group: i64, user: i64) -> Reply<'_, Member>;

    /// 机器人在的群（施工 O-58 上）。
    fn groups(&self) -> Reply<'_, Vec<Group>>;

    /// 机器人的好友（施工 O-58 上）。
    fn friends(&self) -> Reply<'_, Vec<Friend>>;

    /// 群 `group` 的群名，照原样；回了没有的是空的（施工 O-32）。
    fn group_name(&self, group: i64) -> Reply<'_, Option<String>>;

    /// 机器人自己在平台上叫什么，照原样；回了没有的是空的（施工 I-6 中：页面的连接卡）。
    fn nickname(&self) -> Reply<'_, Option<String>>;

    /// 原样的一次调用（`chat.action`，施工 O-29）：平台回了的不管成没成都原样交回。
    ///
    /// # Errors
    ///
    /// 等不到、断了；不会是 [`CallError::Failed`]。
    fn raw<'a>(&'a self, action: &'a str, params: Value) -> Reply<'a, Value>;

    /// 对端是哪个实现：还没问到的是空的。
    fn implementation(&self) -> Option<Implementation>;

    /// 关掉这条连接（同一个号连进来了新的，旧的被顶掉）：在等的都算失败。
    fn close(&self);
}
