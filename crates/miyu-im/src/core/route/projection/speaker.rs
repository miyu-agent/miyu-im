//! 投影里记的说过话的一个人（施工 I-204 从 `projection.rs` 挪出来：那一份行数到了上限）。

use miyu_client::ids::ExternalId;
use miyu_client::time::Timestamp;

/// 说过话的一个人。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Speaker {
    /// 平台上的身份（`by.id`）。
    pub(super) id: ExternalId,
    /// 是不是终端管理员：核心照对应表认出来、记在 `by.owner` 上的（`chat.md` 第七条第 4 条）。
    pub(super) admin: bool,
    /// 这一条记下的时刻（`message.user` 的 `at`）。
    pub(super) at: Timestamp,
    /// 这一条的平台编号（`venue.msg`，O-25 上：引用它）；没有的（照说不会）是空的。
    pub(super) msg: Option<String>,
    /// 这一条引用的平台编号（`venue.reply_to`，O-31：撤回、禁言的目标）；没引用的是空的。
    pub(super) reply_to: Option<String>,
    /// 这一条 @ 了谁（`venue.mentions`，不含她，O-31）。
    pub(super) mentions: Vec<ExternalId>,
    /// 发的人这一条时叫什么（`venue.name`，O-53：冒充提醒照终端管理员用过的名字比）；没写的是空的。
    pub(super) name: Option<String>,
    /// 正文的指纹（群聊内核的 `echo`，I-28：复读照它比）；没有字的是空的。
    pub(super) echo: Option<u64>,
    /// 判过了没有（I-28）：记下的判断里进站链放行了它（`ext.im.chat.decided` 的 `inbound` 是 `pass`）。
    pub(super) passed: bool,
    /// 当过一轮的触发没有（施工 I-89）：主线、支线开轮、并进一轮的 `triggers` 里有它。看一眼只交没当过的。
    pub(super) answered: bool,
    /// 冲她来的没有（施工 I-204）：记下的判断里这一条的条件有「冲她来」（@、引用她、触发词；`ext.im.chat.decided` 的
    /// `conditions` 里有 `direct`）。删群文件这种收不回来的事只听直接叫她的那一条。
    pub(super) direct: bool,
}
