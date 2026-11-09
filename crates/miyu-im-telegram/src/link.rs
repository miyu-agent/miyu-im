//! 一个 Telegram 机器人的连接（宿主的 `Link`，施工 I-183 中上起）：经 Bot API 办宿主要做的事。I-183 中下起发得出字（排队、拆段、
//! 引用、@、话题，[`crate::send`]），答得出群名（收消息时记下的）；图、贴表情、撤回随 I-185，别的照 `Link` 的默认答「做不到」（宿主
//! 照能力表先查）。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::oneshot;

use serde_json::json;

use miyu_im::chat::{
    CallError, Id, Implementation, Link, Member, MenuItem, Outgoing, Pending, Reply, To,
};

use crate::api::Api;
use crate::known::Known;
use crate::send::{Job, LIMIT, Queue, parts};

/// 一个机器人的连接。
#[derive(Debug)]
pub struct Bot {
    /// 机器人的号。
    me: Id,
    /// 机器人的名字：`getMe` 的 `first_name`（加 `last_name`）。
    name: String,
    /// 隐私模式开着（`getMe` 的 `can_read_all_group_messages` 为假）。
    privacy: bool,
    /// 调用口：登记命令菜单这类不排队的调用（I-183 下上）。
    api: Api,
    /// 收消息时记下的（话题、名字、群名）。
    known: Arc<Known>,
    /// 发的那条队。
    queue: Queue,
    /// 关掉了（配置换了、桥在停）：再调的都算断了。
    closed: AtomicBool,
}

impl Bot {
    /// 照 `getMe` 回的号和名字、调用口、记着的、发的那条队造一个。
    pub(crate) fn new(
        (me, name, privacy): (Id, String, bool),
        api: Api,
        known: Arc<Known>,
        queue: Queue,
    ) -> Bot {
        Bot {
            me,
            name,
            privacy,
            api,
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

    fn member<'a>(&'a self, group: &'a Id, user: &'a Id) -> Reply<'a, Member> {
        // 宿主拼「@某人」、认她自己在群里叫什么时问（I-183 下上：原来答不上，@ 她写成了号）。她自己照 getMe 的名字，别人照收消息时记下的；
        // 没见过的名字是空的（宿主照号写）。Telegram 没有群名片，一个人在哪个群都叫一个名字。
        let _ = group;
        let name = if *user == self.me {
            Some(self.name.clone())
        } else {
            self.known.person(user.as_str()).map(|person| person.name)
        };
        let member = Member {
            user: user.clone(),
            name: name.filter(|name| !name.is_empty()),
            rank: None,
            title: None,
        };
        Box::pin(async move { Ok(member) })
    }

    fn menu<'a>(&'a self, owners: &'a [Id], items: &'a [MenuItem]) -> Reply<'a, ()> {
        Box::pin(async move {
            let (commands, names) = crate::menu::build(items);
            self.known.set_commands(names);
            for owner in owners {
                let scope = json!({"type": "chat", "chat_id": owner.as_str().parse::<i64>().map_or_else(|_| json!(owner.as_str()), |id| json!(id))});
                let params = json!({"commands": commands, "scope": scope});
                if let Err(failure) = self
                    .api
                    .call("setMyCommands", &params, std::time::Duration::from_secs(30))
                    .await
                {
                    return Err(match failure {
                        crate::api::Failure::Failed { code, description } => CallError::Failed {
                            said: format!("{code} {description}"),
                            message: description,
                        },
                        _ => CallError::Closed,
                    });
                }
            }
            Ok(())
        })
    }

    fn nickname(&self) -> Reply<'_, Option<String>> {
        let name = (!self.name.is_empty()).then(|| self.name.clone());
        Box::pin(async move { Ok(name) })
    }

    fn hearing(&self, group: &Id) -> Option<serde_json::Value> {
        // 隐私模式开着、又不是管理员的群收不全（问到了是不是管理员才说）；话题群照消息里的 `is_forum`（I-183 下中）。
        let seen = self.known.group(group.as_str())?;
        let partial = self.privacy && seen.admin == Some(false);
        Some(json!({"partial": partial, "topics": seen.forum}))
    }

    fn implementation(&self) -> Option<Implementation> {
        // 对端就是 Telegram，没有「哪个实现」这回事（页面照用户名画，不照实现）。
        None
    }

    fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}
