//! 一个 Telegram 机器人的连接（宿主的 `Link`，施工 I-183 中上起）：经 Bot API 办宿主要做的事。I-183 中下起发得出字（排队、拆段、
//! 引用、@、话题，[`crate::send`]），答得出群名（收消息时记下的）；图、贴表情、撤回随 I-185，别的照 `Link` 的默认答「做不到」（宿主
//! 照能力表先查）。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::oneshot;

use miyu_im::chat::{CallError, Id, Implementation, Link, Outgoing, Pending, Reply, To};

use crate::known::Known;
use crate::send::{Job, LIMIT, Queue, parts};

/// 一个机器人的连接。
#[derive(Debug)]
pub struct Bot {
    /// 机器人的名字：`getMe` 的 `first_name`（加 `last_name`）。
    name: String,
    /// 收消息时记下的（话题、名字、群名）。
    known: Arc<Known>,
    /// 发的那条队。
    queue: Queue,
    /// 关掉了（配置换了、桥在停）：再调的都算断了。
    closed: AtomicBool,
}

impl Bot {
    /// 照 `getMe` 回的名字、记着的、发的那条队造一个。
    pub(crate) fn new(name: String, known: Arc<Known>, queue: Queue) -> Bot {
        Bot {
            name,
            known,
            queue,
            closed: AtomicBool::new(false),
        }
    }
}

/// 发到哪个聊天：私聊的对方、群，照号。
fn chat(to: &To) -> &Id {
    match to {
        To::Private(user) => user,
        To::Group(group) => group,
    }
}

impl Link for Bot {
    fn begin<'a>(&'a self, to: To, outgoing: Outgoing<'a>) -> Reply<'a, Pending> {
        let Outgoing::Text { text, lead } = outgoing else {
            // 图、长文画的图、合并转发、文件随 I-185：交了的答做不到。
            return Box::pin(async { Err(CallError::Unsupported) });
        };
        if self.closed.load(Ordering::Relaxed) {
            return Box::pin(async { Err(CallError::Closed) });
        }
        let (answer, answered) = oneshot::channel();
        let job = Job {
            chat: chat(&to).as_str().to_string(),
            parts: parts(text, lead, &self.known, LIMIT),
            reply: lead.reply.as_deref().and_then(|reply| reply.parse().ok()),
            answer,
        };
        let queued = self.queue.send(job).is_ok();
        Box::pin(async move {
            if !queued {
                return Err(CallError::Closed);
            }
            let pending: Pending =
                Box::pin(async move { answered.await.map_err(|_| CallError::Closed)? });
            Ok(pending)
        })
    }

    fn group_name<'a>(&'a self, group: &'a Id) -> Reply<'a, Option<String>> {
        let title = self.known.title(group.as_str());
        Box::pin(async move { Ok(title) })
    }

    fn nickname(&self) -> Reply<'_, Option<String>> {
        let name = (!self.name.is_empty()).then(|| self.name.clone());
        Box::pin(async move { Ok(name) })
    }

    fn implementation(&self) -> Option<Implementation> {
        // 对端就是 Telegram，没有「哪个实现」这回事（页面照用户名画，不照实现）。
        None
    }

    fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}
