//! 撤回、禁言、戳一戳、踢人（施工 O-31、Q-2，`onebot.md` 第一条「平台工具（一）」第 2 到 6 条；施工 T-2 从宿主挪进驱动）：im 照会话填好场景交
//! 过来（叫她做的那一条、谁叫的、她自己、私聊的对方），照 `target` 定对谁做，认不出目标、参数写错的当场答（施工 I-17：写错的说
//! 哪个参数，私聊里调到只给群的说只给群）；要做的交给 im 另起任务：她给的撤回编号先 `get_msg` 核对在不在这个会话里，再经收进这个会话
//! 的那个机器人号的连接调动作。施工 O-48 起不挡人（谁叫的、禁谁都交给 QQ）。禁言、踢人交给群管理（`moderate`）。结果里写的人照这个群
//! 缓存着的名字，没有的、私聊里的写号（「施工时定的」第 185 条）。

use serde_json::Value;

use miyu_im::chat::{CallError, Id, Link, Msg, To};
use miyu_im::toolset::{Call, Missed, NAME, Planned, clean, message};

use super::args::{bad_args, group_only};
use super::moderate;
use super::target::{Caller, Order, Plan, Scene, Why, plan, recallable};
use crate::onebot::fresh_member;

/// 要调的平台动作（号已经解成整数）。
#[derive(Debug, Clone)]
pub(super) enum Act {
    /// 撤这一条（平台编号原样）；`check` 的先 `get_msg` 核对在不在这个会话里（施工 O-48）。
    Recall { msg: String, check: bool },
    /// 在这个群里禁言这几个号几秒，0 是解禁（施工 I-17 起能几个）。
    Mute {
        group: Id,
        users: Vec<Id>,
        seconds: u64,
    },
    /// 戳这个号；`check` 的是她在群里给的号，先核对是不是群成员（施工 I-32）。
    Poke { user: Id, check: bool },
    /// 把这几个号移出这个群，`block` 的以后不收他们的加群申请（施工 Q-2；I-17 起能几个）。
    Kick {
        group: Id,
        users: Vec<Id>,
        block: bool,
    },
}

/// 撤回、禁言、戳一戳、踢人的一次调用（模块的说明）。私聊里调到禁言、踢人（`venues` 不给，照说不会）的答只给群；她自己是谁 im 没交
/// 过来的（照说不会）答够不着。禁言、踢人归群管理（`moderate`）：当场挡下的答它说的，做成了记进管理记录。
pub(super) fn acting(call: Call) -> Planned {
    let Some(order) = Order::read(&call.tool, &call.args) else {
        return Planned::Now(call.answers.say("unreachable", &[], true));
    };
    if matches!(call.to, To::Private(_)) && matches!(order, Order::Mute { .. } | Order::Kick { .. })
    {
        return group_only(&call);
    }
    let Some(decided) = decide(&call, order) else {
        return Planned::Now(call.answers.say("unreachable", &[], true));
    };
    let act = match act(&call, decided) {
        Ok(act) => act,
        Err(Why::BadSeconds) => return Planned::Now(moderate::bad_seconds(&call)),
        Err(Why::Bad(bad)) => return bad_args(&call, bad),
        Err(why) => return Planned::Now(call.answers.say(why.name(), &[], true)),
    };
    if matches!(act, Act::Mute { .. } | Act::Kick { .. }) {
        return moderate::moderated(call, act);
    }
    Planned::later(async move {
        let outcome = done(&call, &act).await;
        answered(&call, &act, outcome)
    })
}

/// 对谁做：群里照叫她做的那一条，私聊里照对方、最近一条的引用。她自己是谁不知道的是空的。
fn decide(call: &Call, order: Order) -> Option<Plan> {
    let me = call.me.as_ref()?;
    let caller = Caller {
        id: call.by.clone(),
    };
    let scene = Scene {
        peer: call.other.as_ref(),
        me,
        platform: call.platform,
    };
    Some(plan(order, &caller, call.origin.as_ref(), &scene))
}

/// 定下来的 `plan` 换成要调的动作：号解出来。认不出目标的交回为什么。
fn act(call: &Call, plan: Plan) -> Result<Act, Why> {
    let number = |user: &miyu_client::ids::ExternalId| {
        miyu_chat::parse_person(user)
            .and_then(|(_, number)| Id::new(number))
            .ok_or(Why::NoTarget)
    };
    let group = || match call.to.clone() {
        To::Group(group) => Ok(group),
        To::Private(_) => Err(Why::NoTarget),
    };
    match plan {
        Plan::Refuse(why) => Err(why),
        Plan::Recall { msg, check } => Ok(Act::Recall { msg, check }),
        Plan::Poke(user) => Ok(Act::Poke {
            user: number(&user)?,
            check: matches!(call.to, To::Group(_)) && !call.args["user"].is_null(),
        }),
        Plan::Mute { users, seconds } => Ok(Act::Mute {
            group: group()?,
            users: users.iter().map(number).collect::<Result<_, _>>()?,
            seconds,
        }),
        Plan::Kick { users, block } => Ok(Act::Kick {
            group: group()?,
            users: users.iter().map(number).collect::<Result<_, _>>()?,
            block,
        }),
    }
}

/// 没做成的：答的那一句的名字（`failed`、`unreachable`、`unanswered`；她给的撤回编号不在这个会话里的 `not-found`；施工 I-17 踢人的
/// `not-members`、`protected`），`failed` 的带平台说的原因。
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Undone(pub(super) &'static str, pub(super) Option<String>);

/// 调撤回、戳一戳，经那个机器人号这时的连接；她给的撤回编号先 `get_msg` 核对（施工 O-48）：平台说没有、不在这个会话里的答
/// `not-found`。禁言、踢人在群管理（`moderate`），到不了这里。成了的交回现问群成员时知道的名字（施工 I-32，没问的是空的）。
async fn done(call: &Call, act: &Act) -> Result<Option<String>, Undone> {
    let Some(link) = &call.link else {
        return Err(Undone("unreachable", None));
    };
    let result = match act {
        Act::Recall { msg, check } => {
            if *check {
                let fetched = match message(link.as_ref(), msg).await {
                    Ok(fetched) => fetched,
                    Err(Missed::Failed(_)) => return Err(Undone("not-found", None)),
                    Err(Missed::Unanswered) => return Err(Undone("unanswered", None)),
                    Err(_) => return Err(Undone("unreachable", None)),
                };
                if !recallable(&fetched, &call.to, &call.bot) {
                    return Err(Undone("not-found", None));
                }
            }
            link.recall(Msg::Text(msg)).await
        }
        Act::Poke { user, check } => {
            let name = if *check {
                match member(call, link.as_ref(), user.clone()).await? {
                    Some(name) => name,
                    None => return Err(Undone("not-members", Some(user.to_string()))),
                }
            } else {
                None
            };
            return link
                .poke(call.to.clone(), user)
                .await
                .map(|()| name)
                .map_err(undone);
        }
        Act::Mute { .. } | Act::Kick { .. } => return Err(Undone("unreachable", None)),
    };
    result.map(|()| None).map_err(undone)
}

/// 答的那一句：成了的照动作说，没成的说为什么。
fn answered(call: &Call, act: &Act, outcome: Result<Option<String>, Undone>) -> Value {
    let who = |user: Id| {
        call.names
            .get(&user)
            .cloned()
            .unwrap_or_else(|| user.to_string())
    };
    match (outcome, act) {
        (Ok(learned), Act::Poke { user, .. }) => {
            let who = learned.unwrap_or_else(|| who(user.clone()));
            call.answers.say("poked", &[("who", &who)], false)
        }
        // 她给的号不在群里（施工 I-32）：照 `mention` 答哪个号。
        (Err(Undone("not-members", Some(number))), _) => {
            call.answers.say("not-members", &[("who", &number)], true)
        }
        (Ok(_), _) => call.answers.say("recalled", &[], false),
        (Err(Undone(why, detail)), _) => {
            let fields: Vec<(&str, &str)> = detail
                .as_deref()
                .map(|detail| ("detail", detail))
                .into_iter()
                .collect();
            call.answers.say(why, &fields, true)
        }
    }
}

/// `user` 是不是这个群的成员（施工 I-32）：缓存着名字的算是，没有的现问一次（同踢人的 `fresh_member`）。是的交回现问到的名字
/// （洗过；缓存着的、没名字的是空的，照缓存、号写），不是的是空的。私聊的不问。
async fn member(call: &Call, link: &dyn Link, user: Id) -> Result<Option<Option<String>>, Undone> {
    let To::Group(group) = call.to.clone() else {
        return Ok(Some(None));
    };
    if call.names.contains_key(&user) {
        return Ok(Some(None));
    }
    match fresh_member(link, &group, &user).await {
        Ok(found) => {
            Ok(found.map(|member| member.name.as_deref().and_then(|name| clean(name, NAME))))
        }
        // NapCat 对不在群里的人回失败（同踢人）。
        Err(CallError::Failed { .. }) => Ok(None),
        Err(error) => Err(undone(error)),
    }
}

/// 平台没成算成哪一句：回了失败的是 `failed`，原因照驱动读好的原话（`said`）；等不到是 `unanswered`；没连着、断了是 `unreachable`。
pub(super) fn undone(error: CallError) -> Undone {
    match error {
        CallError::Failed { said, .. } => Undone("failed", Some(said)),
        CallError::Timeout => Undone("unanswered", None),
        CallError::Closed => Undone("unreachable", None),
    }
}

#[cfg(test)]
mod tests;
