//! 宿主测试用的假连接（施工 I-2 下）：宿主要它做的每一件放进一条队，测试照先后取出来看、答成没成。只认测试用到的几件，别的
//! 一律当断了。

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use super::{
    CallError, Fetch, Fetched, Friend, Group, Implementation, Link, Located, Member, Msg, Node,
    Outgoing, Pending, Reply, To,
};

/// 要它做的一件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Did {
    /// 撤回：编号照 `Msg` 的调试写法（`Number(77)`、`Text("77")`）。
    Recall(String),
    /// 贴（`on`）、摘表情。
    React {
        /// 平台编号。
        message: String,
        /// 表情。
        emoji: String,
        /// 贴的是真。
        on: bool,
    },
}

/// 一件在等答的。
#[derive(Debug)]
pub(crate) struct Asked {
    /// 要做的。
    pub(crate) did: Did,
    /// 交答的那一头。
    answer: oneshot::Sender<Result<(), CallError>>,
}

impl Asked {
    /// 答：`ok` 的成了，不然平台回了没成。
    pub(crate) fn answer(self, ok: bool) {
        let answer = match ok {
            true => Ok(()),
            false => Err(CallError::Failed {
                said: "no".to_string(),
                message: "no".to_string(),
            }),
        };
        // 等的一头放下了（任务停了）就不答。
        let _ = self.answer.send(answer);
    }
}

/// 假连接。
#[derive(Debug)]
pub(crate) struct Fake {
    asked: mpsc::Sender<Asked>,
}

/// 一条假连接和它收到的要做的事。
pub(crate) fn fake() -> (Arc<Fake>, mpsc::Receiver<Asked>) {
    let (asked, received) = mpsc::channel(8);
    (Arc::new(Fake { asked }), received)
}

/// 收到的下一件：钟停着，一直没有的，过一小时（停住的钟上的）就算没有，不卡住。
pub(crate) async fn next(received: &mut mpsc::Receiver<Asked>) -> Asked {
    tokio::time::timeout(Duration::from_secs(3600), received.recv())
        .await
        .expect("要做了一件")
        .expect("连接还在")
}

impl Fake {
    /// 放进队，等答。
    fn ask(&self, did: Did) -> Reply<'_, ()> {
        Box::pin(async move {
            let (answer, answered) = oneshot::channel();
            self.asked
                .send(Asked { did, answer })
                .await
                .map_err(|_| CallError::Closed)?;
            answered.await.map_err(|_| CallError::Closed)?
        })
    }
}

/// 不认的一件：当断了。
fn closed<'a, T: 'a>() -> Reply<'a, T> {
    Box::pin(async { Err(CallError::Closed) })
}

impl Link for Fake {
    fn begin<'a>(&'a self, _: To, _: Outgoing<'a>) -> Reply<'a, Pending> {
        closed()
    }
    fn recall<'a>(&'a self, message: Msg<'a>) -> Reply<'a, ()> {
        self.ask(Did::Recall(format!("{message:?}")))
    }
    fn mute(&self, _: i64, _: i64, _: u64) -> Reply<'_, ()> {
        closed()
    }
    fn poke(&self, _: To, _: i64) -> Reply<'_, ()> {
        closed()
    }
    fn react<'a>(&'a self, message: &'a str, emoji: &'a str, on: bool) -> Reply<'a, ()> {
        self.ask(Did::React {
            message: message.to_string(),
            emoji: emoji.to_string(),
            on,
        })
    }
    fn befriend<'a>(&'a self, _: &'a str) -> Reply<'a, ()> {
        closed()
    }
    fn message<'a>(&'a self, _: &'a str) -> Reply<'a, Fetched> {
        closed()
    }
    fn fetch<'a>(&'a self, _: Fetch, _: &'a str, _: Duration) -> Reply<'a, Located> {
        closed()
    }
    fn forwarded<'a>(&'a self, _: &'a str) -> Reply<'a, Option<Vec<Node>>> {
        closed()
    }
    fn members(&self, _: i64) -> Reply<'_, Vec<Member>> {
        closed()
    }
    fn member(&self, _: i64, _: i64) -> Reply<'_, Member> {
        closed()
    }
    fn groups(&self) -> Reply<'_, Vec<Group>> {
        closed()
    }
    fn friends(&self) -> Reply<'_, Vec<Friend>> {
        closed()
    }
    fn group_name(&self, _: i64) -> Reply<'_, Option<String>> {
        closed()
    }
    fn raw<'a>(&'a self, _: &'a str, _: Value) -> Reply<'a, Value> {
        closed()
    }
    fn implementation(&self) -> Option<Implementation> {
        None
    }
    fn close(&self) {}
}
