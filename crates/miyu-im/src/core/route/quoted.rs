//! 引用的那一条的内容（施工 I-238，核心 QT-1，`venues.md`「场所的格」的 `quoted`；私聊的施工 I-245，核心 QT-2）：群里、私聊里
//! 一条消息引用了别的一条，交 `session.send` 以前把那一条取回来，照合并转发里一条的写法写进场所的格的 `quoted`，核心写在
//! `reply-to` 那一行下面。那一条在不在她的日志里都取：写不写由核心定。
//!
//! - 怎么取：经收进这条消息的那个机器人号现在的连接调 `get_msg`（Telegram 照收的时候记下的答，施工 I-185 上），等一次调用
//!   （`call_timeout_seconds`）。取不到的不写，记一行 `WARN quote not fetched venue=… message=… quoted=… why=…`，这一条只有编号。
//! - 取回来的不是这个聊天的不写（`why=elsewhere`）：群里的照群号；私聊的要是私聊、发的人是对方或者收进这条消息的机器人号。
//!   别处的话不往这里带。
//! - 名字照发的人当时的群名片、昵称，没有的写号；正文、@ 的名字、带的东西、里面的转发同转发里的一条（`forwards`），里面的转发从
//!   第一层数、照 `forward_depth`。和这条消息的转发合起来算核心的整条上限，转发先放，放不下的不写（`why=full`）。
//! - 带的东西照那一条自己的编号数（施工 I-245，2026-11-16 项目主人定）：从 1 数，再数里面的转发，和 `fetch_media` 照那一条的
//!   `msg` 数的一个数法，她没叫到的时候也能自己取来看。她取不到的（私聊里她自己说的，`fetch_media` 当没有）不编。冲她来的这一条，
//!   引用的那一条的图照旧另取原图带上（`pictures`）。

use serde_json::Value;

use super::Route;
use super::forwards::{At, Budget};
use crate::TARGET;
use crate::chat::{CallError, Fetched, Node};
use crate::media::{in_chat, owned};
use crate::toolset::Numbering;

impl Route {
    /// 取 `at` 那个聊天里平台编号是 `id` 的那一条，照转发里的一条写好、算进 `budget`（施工 I-238）。带的东西照那一条自己的编号
    /// 数（施工 I-245）。取不到的、不是这个聊天的、没名字的、放不下的交回空的，记一行运行日志。
    pub(super) async fn quoted(&self, at: &At<'_>, id: &str, budget: &mut Budget) -> Option<Value> {
        let why = match Self::fetch_quoted(at, id).await {
            Ok(fetched) => {
                // 她照那一条的 `msg` 和编号 `fetch_media`，取不到的（私聊里她自己说的，[`owned`]）不编。
                let mut numbers = Numbering::after(0);
                if !owned(&fetched, at.to.clone()) {
                    numbers.stop();
                }
                let read = Node {
                    user: fetched.user,
                    name: fetched.name,
                    segments: fetched.segments,
                };
                let this = std::mem::replace(&mut budget.numbers, numbers);
                let placed = self.node(at, &read, 0, budget).await;
                budget.numbers = this;
                match placed {
                    Ok(one) => return Some(one),
                    Err(false) => "nameless",
                    Err(true) => "full",
                }
            }
            Err(why) => why,
        };
        tracing::warn!(target: TARGET, venue = at.venue, message = at.msg, quoted = id, why, "quote not fetched");
        None
    }

    /// 经 `at` 的连接取那一条；这个聊天的才交回（[`in_chat`]）。这一条里前面的转发取不到的照样问一次：过期的转发和这一条不相干。
    async fn fetch_quoted(at: &At<'_>, id: &str) -> Result<Fetched, &'static str> {
        let Some(link) = &at.link else {
            return Err("unreachable");
        };
        let fetched = match link.message(&at.to, id).await {
            Ok(fetched) => fetched,
            Err(CallError::Failed { .. }) => return Err("failed"),
            Err(CallError::Timeout) => return Err("unanswered"),
            Err(CallError::Closed) => return Err("unreachable"),
            Err(CallError::Unsupported) => return Err("unsupported"),
        };
        match in_chat(&fetched, &at.to, &at.bot) {
            true => Ok(fetched),
            false => Err("elsewhere"),
        }
    }
}
