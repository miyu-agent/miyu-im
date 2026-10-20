//! 撤回、禁言、戳一戳、踢人（施工 O-31、Q-2，`onebot.md` 第一条「平台工具（一）」第 2 到 6 条；施工 T-2 从宿主挪进驱动）：im 照会话填好场景交
//! 过来（叫她做的那一条、谁叫的、她自己、私聊的对方），照 `target` 定对谁做，认不出目标、参数写错的当场答；要做的交给 im 另起任务：她给
//! 的撤回编号先 `get_msg` 核对在不在这个会话里，再经收进这个会话的那个机器人号的连接调动作。施工 O-48 起不挡人（谁叫的、禁谁都交给
//! QQ）。结果里写的人照这个群缓存着的名字，没有的、私聊里的写号（「施工时定的」第 185 条）。

use serde_json::Value;

use miyu_im::chat::{CallError, Msg, To};
use miyu_im::toolset::{Call, Missed, Planned, duration, message};

use super::moderate;
use super::target::{Caller, Order, Plan, Scene, Why, plan, recallable};

/// 要调的平台动作（号已经解成整数）。
#[derive(Debug, Clone)]
pub(super) enum Act {
    /// 撤这一条（平台编号原样）；`check` 的先 `get_msg` 核对在不在这个会话里（施工 O-48）。
    Recall { msg: String, check: bool },
    /// 在这个群里禁言这个号几秒，0 是解禁。
    Mute { group: i64, user: i64, seconds: u64 },
    /// 戳这个号。
    Poke(i64),
    /// 把这个号移出这个群，`block` 的以后不收他的加群申请（施工 Q-2）。
    Kick { group: i64, user: i64, block: bool },
}

/// 撤回、禁言、戳一戳、踢人的一次调用（模块的说明）。参数写错、私聊里调到禁言、踢人（`venues` 不给，照说不会）的照不认识的工具答。
/// 禁言、踢人归群管理（`moderate`）：当场挡下的答它说的，做成了记进管理记录。
pub(super) fn acting(call: Call) -> Planned {
    let decided = Order::read(&call.tool, &call.args).and_then(|order| decide(&call, order));
    let Some(decided) = decided else {
        return Planned::Now(call.answers.say("unknown", &[("name", &call.tool)], true));
    };
    let act = match act(&call, decided) {
        Ok(act) => act,
        Err(Why::BadSeconds) => return Planned::Now(moderate::bad_seconds(&call)),
        Err(why) => return Planned::Now(call.answers.say(why.name(), &[], true)),
    };
    if let Some(answer) = moderate::checked(&call, &act) {
        return Planned::Now(answer);
    }
    Planned::later(async move {
        let outcome = done(&call, &act).await;
        if outcome.is_ok() {
            moderate::recorded(&call, &act).await;
        }
        answered(&call, &act, outcome)
    })
}

/// 对谁做：群里照叫她做的那一条，私聊里照对方、最近一条的引用。私聊里的禁言是空的。
fn decide(call: &Call, order: Order) -> Option<Plan> {
    let me = call.me.as_ref()?;
    if matches!(call.to, To::Private(_)) && matches!(order, Order::Mute { .. } | Order::Kick { .. })
    {
        return None;
    }
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

/// 定下来的 `plan` 换成要调的动作：号解成整数。认不出目标的交回为什么。
fn act(call: &Call, plan: Plan) -> Result<Act, Why> {
    let number = |user: &miyu_client::ids::ExternalId| {
        miyu_chat::parse_person(user)
            .and_then(|(_, number)| number.parse::<i64>().ok())
            .ok_or(Why::NoTarget)
    };
    match plan {
        Plan::Refuse(why) => Err(why),
        Plan::Recall { msg, check } => Ok(Act::Recall { msg, check }),
        Plan::Poke(user) => Ok(Act::Poke(number(&user)?)),
        Plan::Mute { user, seconds } => {
            let user = number(&user)?;
            let To::Group(group) = call.to else {
                return Err(Why::NoTarget);
            };
            Ok(Act::Mute {
                group,
                user,
                seconds,
            })
        }
        Plan::Kick { user, block } => {
            let user = number(&user)?;
            let To::Group(group) = call.to else {
                return Err(Why::NoTarget);
            };
            Ok(Act::Kick { group, user, block })
        }
    }
}

/// 没做成的：答的那一句的名字（`failed`、`unreachable`、`unanswered`；她给的撤回编号不在这个会话里的 `not-found`），`failed` 的
/// 带平台说的原因。
#[derive(Debug, PartialEq, Eq)]
struct Undone(&'static str, Option<String>);

/// 调动作，经那个机器人号这时的连接；她给的撤回编号先 `get_msg` 核对（施工 O-48）：平台说没有、不在这个会话里的答 `not-found`。
async fn done(call: &Call, act: &Act) -> Result<(), Undone> {
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
                if !recallable(&fetched, call.to, call.bot) {
                    return Err(Undone("not-found", None));
                }
            }
            link.recall(Msg::Text(msg)).await
        }
        Act::Mute {
            group,
            user,
            seconds,
        } => {
            // 她在这个群里不是管理员、群主的，群管理不生效（施工 Q-1）：问不到身份的照问不到答。
            if !moderate::admin_here(link.as_ref(), *group, call.bot)
                .await
                .map_err(undone)?
            {
                return Err(Undone(moderate::ADMIN_OFF, None));
            }
            link.mute(*group, *user, *seconds).await
        }
        Act::Kick { group, user, block } => {
            if !moderate::admin_here(link.as_ref(), *group, call.bot)
                .await
                .map_err(undone)?
            {
                return Err(Undone(moderate::ADMIN_OFF, None));
            }
            link.kick(*group, *user, *block).await
        }
        Act::Poke(user) => link.poke(call.to, *user).await,
    };
    result.map_err(undone)
}

/// 答的那一句：成了的照动作说，没成的说为什么。
fn answered(call: &Call, act: &Act, outcome: Result<(), Undone>) -> Value {
    let who = |user: i64| {
        call.names
            .get(&user)
            .cloned()
            .unwrap_or_else(|| user.to_string())
    };
    match (outcome, act) {
        (Ok(()), Act::Recall { .. }) => call.answers.say("recalled", &[], false),
        (
            Ok(()),
            Act::Mute {
                seconds: 0, user, ..
            },
        ) => call.answers.say("unmuted", &[("who", &who(*user))], false),
        (Ok(()), Act::Mute { seconds, user, .. }) => call.answers.say(
            "muted",
            &[("who", &who(*user)), ("duration", &duration(*seconds))],
            false,
        ),
        (Ok(()), Act::Poke(user)) => call.answers.say("poked", &[("who", &who(*user))], false),
        (Ok(()), Act::Kick { user, block, .. }) => {
            let said = if *block { "kicked-blocked" } else { "kicked" };
            call.answers.say(said, &[("who", &who(*user))], false)
        }
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

/// 平台没成算成哪一句：回了失败的是 `failed`，原因照驱动读好的原话（`said`）；等不到是 `unanswered`；没连着、断了是 `unreachable`。
fn undone(error: CallError) -> Undone {
    match error {
        CallError::Failed { said, .. } => Undone("failed", Some(said)),
        CallError::Timeout => Undone("unanswered", None),
        CallError::Closed => Undone("unreachable", None),
    }
}

#[cfg(test)]
mod tests;
