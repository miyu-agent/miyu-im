//! 引用的那一条的内容（施工 I-238，核心 QT-1，`venues.md`「场所的格」的 `quoted`）：群里一条消息引用了别的一条，交
//! `session.send` 以前把那一条取回来，照合并转发里一条的写法写进场所的格的 `quoted`。她的日志里这一条以前有那一条的，核心照旧
//! 只写编号；没有的（会话新开的、早挤出去的、桥没收到的）写在 `reply-to` 那一行下面。判在核心，桥只要有引用就取。
//!
//! - 怎么取：经收进这条消息的那个机器人号现在的连接调 `get_msg`（Telegram 照收的时候记下的答，施工 I-185 上），等一次调用
//!   （`call_timeout_seconds`）。取不到的不写，记一行 `WARN quote not fetched venue=… message=… quoted=… why=…`，这一条只有编号。
//! - 取回来的不是这个群的（私聊的、别的群的）不写（`why=elsewhere`）：别的群的话不往这个群带。
//! - 名字照发的人当时的群名片、昵称，没有的写号；正文、@ 的名字、带的东西、里面的转发同转发里的一条（`forwards`），里面的转发从
//!   第一层数、照 `forward_depth`。和这条消息的转发合起来算核心的整条上限，转发先放，放不下的不写（`why=full`）。
//! - 带的东西不编号：她照这条消息的 `msg` 和编号 `fetch_media` 取的是这一条和它的转发里的，引用的那一条的另有它自己的编号。
//!   冲她来的这一条，引用的那一条的图照旧另取原图带上（`pictures`）。

use serde_json::Value;

use super::Route;
use super::forwards::{At, Budget};
use crate::TARGET;
use crate::chat::{CallError, Fetched, Node, To};

impl Route {
    /// 取 `at` 那个群里平台编号是 `id` 的那一条，照转发里的一条写好、算进 `budget`（施工 I-238）。取不到的、不是这个群的、
    /// 没名字的、放不下的交回空的，记一行运行日志。
    pub(super) async fn quoted(&self, at: &At<'_>, id: &str, budget: &mut Budget) -> Option<Value> {
        let why = match Self::fetch_quoted(at, id).await {
            Ok(fetched) => {
                let read = Node {
                    user: fetched.user,
                    name: fetched.name,
                    segments: fetched.segments,
                };
                budget.numbers.stop();
                match self.node(at, &read, 0, budget).await {
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

    /// 经 `at` 的连接取那一条；这个群的才交回。这一条里前面的转发取不到的照样问一次：过期的转发和这一条不相干。
    async fn fetch_quoted(at: &At<'_>, id: &str) -> Result<Fetched, &'static str> {
        let (Some(link), Some(group)) = (&at.link, &at.group) else {
            return Err("unreachable");
        };
        let fetched = match link.message(&To::Group(group.clone()), id).await {
            Ok(fetched) => fetched,
            Err(CallError::Failed { .. }) => return Err("failed"),
            Err(CallError::Timeout) => return Err("unanswered"),
            Err(CallError::Closed) => return Err("unreachable"),
            Err(CallError::Unsupported) => return Err("unsupported"),
        };
        if fetched.private || fetched.group.as_ref() != Some(group) {
            return Err("elsewhere");
        }
        Ok(fetched)
    }
}
