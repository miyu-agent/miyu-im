//! 平台工具（一）对谁做（施工 O-31，`onebot.md` 第一条「平台工具（一）」第 3、4 条；18 第十一节；施工 T-2 从宿主挪进驱动）：纯逻辑。
//!
//! 施工 O-48（2026-10-17 项目主人定：工具只做动作，做不做由她判断，程序不替她判断）起不挡人：谁叫的都做，动不动得了由 QQ 说。
//! 这里只认目标：撤回撤她给的 `msg`，没给的撤叫她做的那条引用的那一条；禁言禁她给的 `user`，没给的禁那条引用的人或者唯一 @
//! 的人；戳一戳戳 @ 的那一个，没有的戳叫她的人（[`Origin`]：群里是投影里主线这一轮她回的那一条，私聊是最近一条人说的话）。
//! 叫她做的那条和 `tool.call` 的 `by` 对不上的当没有：A 的身份配不上 B 的引用（「施工时定的」第 179 条）。她给的 `msg` 先核对
//! 在不在这个会话里（[`recallable`]，同取图）：编号是数字，人打得出别的群的。

use miyu_client::ids::ExternalId;
use serde_json::Value;

use miyu_im::chat::{Fetched, Platform, To};
use miyu_im::toolset::{Origin, msg_id};

use super::{KICK, MUTE, POKE, RECALL};

/// 禁言最多多少秒：QQ 的上限，30 天（施工单「要定的」第 2 条）。是平台定的，不是桥的参数。
const MAX_SECONDS: u64 = 2_592_000;

/// 她给的一个参数：没给、给了、给错了（施工 O-48）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Arg<T> {
    /// 没给：照叫她做的那条认。
    Absent,
    /// 给了。
    Given(T),
    /// 给了，写法不对。
    Bad,
}

impl<T> Arg<T> {
    /// `value` 照 `read` 读：`null` 是没给，读不出的是给错了。
    fn read(value: &Value, read: impl Fn(&Value) -> Option<T>) -> Arg<T> {
        match value {
            Value::Null => Arg::Absent,
            value => read(value).map_or(Arg::Bad, Arg::Given),
        }
    }
}

/// 叫她做的一件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Order {
    /// 撤回：她给的消息编号（施工 O-48）。
    Recall(Arg<String>),
    /// 禁言。
    Mute {
        /// 几秒（0 是解禁）；没写、不是整数、出了范围的是空的。
        seconds: Option<u64>,
        /// 她给的 QQ 号（施工 O-48）。
        user: Arg<i64>,
    },
    /// 戳一戳。
    Poke,
    /// 移出群（施工 Q-2）：她给的 QQ 号，以后不收他的加群申请没有。
    Kick {
        /// 她给的 QQ 号。
        user: Arg<i64>,
        /// 以后不收他的加群申请。
        block: bool,
    },
}

impl Order {
    /// 工具 `tool`、参数 `args`：不是这几件的是空的。
    pub(crate) fn read(tool: &str, args: &Value) -> Option<Order> {
        match tool {
            RECALL => Some(Order::Recall(Arg::read(&args["msg"], msg_id))),
            MUTE => {
                let seconds = args["seconds"]
                    .as_u64()
                    .filter(|seconds| *seconds <= MAX_SECONDS);
                let user = Arg::read(&args["user"], qq_number);
                Some(Order::Mute { seconds, user })
            }
            POKE => Some(Order::Poke),
            KICK => Some(Order::Kick {
                user: Arg::read(&args["user"], qq_number),
                block: args["block"].as_bool().unwrap_or(false),
            }),
            _ => None,
        }
    }
}

/// QQ 号：正整数，或者写成正整数的字。
fn qq_number(value: &Value) -> Option<i64> {
    let number = match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    };
    number.filter(|number| *number > 0)
}

/// 谁叫的（im 照 `tool.call` 的 `by` 认好交过来，`miyu_im::toolset::Call::by`）。没有触发的回合、本机的人是空的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Caller {
    /// 平台上的身份。
    pub(crate) id: Option<ExternalId>,
}

/// 这个场所的情形。
pub(crate) struct Scene<'a> {
    /// 私聊的对方；群的是空的。
    pub(crate) peer: Option<&'a ExternalId>,
    /// 她自己（收进这个会话的机器人号）。
    pub(crate) me: &'a ExternalId,
    /// 平台：她给的号照它拼成平台上的人。
    pub(crate) platform: Platform,
}

/// 定下来的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Plan {
    /// 撤平台编号是 `msg` 的那一条；`check` 的是她给的编号，先核对在不在这个会话里。
    Recall {
        /// 平台编号。
        msg: String,
        /// 先核对。
        check: bool,
    },
    /// 禁言这个人这么多秒（0 是解禁）。
    Mute {
        /// 禁谁。
        user: ExternalId,
        /// 几秒。
        seconds: u64,
    },
    /// 戳这个人。
    Poke(ExternalId),
    /// 把这个人移出群（施工 Q-2）。
    Kick {
        /// 踢谁。
        user: ExternalId,
        /// 以后不收他的加群申请。
        block: bool,
    },
    /// 不做，回一句。
    Refuse(Why),
}

/// 做不了的为什么：每一种答一句（`tool-results/<名字>.txt`，「平台工具（一）」第 5 条）。都是认不出目标、参数写错，不是不让做。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Why {
    /// 撤回：没给 `msg`，叫她做的那条又没引用（或者没有那条）。
    NoQuote,
    /// 撤回：`msg` 写错了，或者不在这个会话里。
    NotFound,
    /// 禁言、戳一戳：认不出人（`user` 写错的也是）。
    NoTarget,
    /// 禁言、戳一戳：不止一个人。
    ManyTargets,
    /// 禁言：秒数没写、不是整数、出了范围。
    BadSeconds,
}

impl Why {
    /// 答的那一句的名字。
    pub(crate) fn name(self) -> &'static str {
        match self {
            Why::NoQuote => "no-quote",
            Why::NotFound => "not-found",
            Why::NoTarget => "no-target",
            Why::ManyTargets => "many-targets",
            Why::BadSeconds => "bad-seconds",
        }
    }
}

/// 定 `order` 对谁做：`caller` 叫的，叫她做的那条是 `origin`，场所的情形 `scene`。私聊里的禁言（`venues` 不给）由调的一方挡在
/// 前面。
pub(crate) fn plan(order: Order, caller: &Caller, origin: Option<&Origin>, scene: &Scene) -> Plan {
    let origin = origin.filter(|origin| caller.id.as_ref() == Some(&origin.sender));
    match order {
        Order::Recall(msg) => recall(msg, origin),
        Order::Mute { seconds, user } => mute(seconds, user, origin, scene),
        Order::Poke => poke(caller, origin, scene),
        Order::Kick { user, block } => match person(user, origin, scene) {
            Ok(user) => Plan::Kick { user, block },
            Err(why) => Plan::Refuse(why),
        },
    }
}

/// 撤回：她给的编号（先核对），没给的撤引用的那一条。
fn recall(msg: Arg<String>, origin: Option<&Origin>) -> Plan {
    match msg {
        Arg::Given(msg) => Plan::Recall { msg, check: true },
        Arg::Bad => Plan::Refuse(Why::NotFound),
        Arg::Absent => match origin.and_then(|origin| origin.quote.as_ref()) {
            Some(quote) => Plan::Recall {
                msg: quote.msg.clone(),
                check: false,
            },
            None => Plan::Refuse(Why::NoQuote),
        },
    }
}

/// 禁言：先看秒数，再认人：她给的号，没给的照引用的人和 @ 的人认唯一的一个。
fn mute(seconds: Option<u64>, user: Arg<i64>, origin: Option<&Origin>, scene: &Scene) -> Plan {
    let Some(seconds) = seconds else {
        return Plan::Refuse(Why::BadSeconds);
    };
    match person(user, origin, scene) {
        Ok(user) => Plan::Mute { user, seconds },
        Err(why) => Plan::Refuse(why),
    }
}

/// 禁言、踢人对谁：她给的号，没给的照引用的人和 @ 的人认唯一的一个。
fn person(user: Arg<i64>, origin: Option<&Origin>, scene: &Scene) -> Result<ExternalId, Why> {
    match user {
        Arg::Given(number) => scene.platform.person(number).map_err(|_| Why::NoTarget),
        Arg::Bad => Err(Why::NoTarget),
        Arg::Absent => {
            // 她的那一条没有发的人（她说的不是人说的话），去掉她的那一步在 `one` 里。
            let quoted = origin
                .and_then(|origin| origin.quote.as_ref())
                .and_then(|quote| quote.sender.as_ref());
            let mentions = origin
                .map(|origin| origin.mentions.as_slice())
                .unwrap_or_default();
            one(quoted.into_iter().chain(mentions), scene.me)
        }
    }
}

/// 戳一戳：私聊戳对方；群里 @ 的那一个，没有的戳叫她的人。
fn poke(caller: &Caller, origin: Option<&Origin>, scene: &Scene) -> Plan {
    if let Some(peer) = scene.peer {
        return Plan::Poke(peer.clone());
    }
    let mentions = origin
        .map(|origin| origin.mentions.as_slice())
        .unwrap_or_default();
    match one(mentions, scene.me) {
        Ok(user) => Plan::Poke(user),
        Err(Why::NoTarget) => match &caller.id {
            Some(id) => Plan::Poke(id.clone()),
            None => Plan::Refuse(Why::NoTarget),
        },
        Err(why) => Plan::Refuse(why),
    }
}

/// 从 `people` 里挑出唯一的一个人：去掉她自己 `me`、重的；一个都没有是 [`Why::NoTarget`]，不止一个是 [`Why::ManyTargets`]。
fn one<'a>(
    people: impl IntoIterator<Item = &'a ExternalId>,
    me: &ExternalId,
) -> Result<ExternalId, Why> {
    let mut found: Vec<&ExternalId> = Vec::new();
    for person in people {
        if person != me && !found.contains(&person) {
            found.push(person);
        }
    }
    match found.as_slice() {
        [] => Err(Why::NoTarget),
        [only] => Ok((*only).clone()),
        _ => Err(Why::ManyTargets),
    }
}

/// 她给的编号取回来的那一条撤不撤得到（施工 O-48，同取图核对归属）：群里是这个群的，她的、别人的都算；私聊里是私聊的，发的人
/// 是对方或者她（机器人号 `bot`）。
pub(crate) fn recallable(fetched: &Fetched, to: To, bot: i64) -> bool {
    match to {
        To::Group(group) => fetched.group == Some(group),
        To::Private(user) => {
            fetched.private && (fetched.user == Some(user) || fetched.user == Some(bot))
        }
    }
}

#[cfg(test)]
mod tests;
