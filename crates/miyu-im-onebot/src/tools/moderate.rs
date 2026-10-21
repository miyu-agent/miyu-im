//! 禁言、踢人归群管理（施工 Q-1、Q-2，设计 02 第四节）：对谁做照 `target`（`acting` 认好交过来），这里是群管理加的那几样：插件在这个
//! 群开着、秒数照设置的最短最长（[`checked`]），要做之前问她在这个群的身份（`admin::admin_here`，现问），做成了记进管理记录
//! （[`recorded`]，记录开关关了的不记；AI 自己做的不另给会话记事实：工具结果已经在她的上下文里）。
//!
//! 施工 I-17（照旧版）：
//!
//! - 一次能管几个人（`users`，最多 32 个）：一个个做、一个个报，成了几个、哪些没成、为什么（[`answered`]）。
//! - 踢人不踢群主、管理员，NapCat 回失败时核实人走没走（`kick`）。
//!
//! 施工 I-20（2026-10-21 项目主人定）：踢人不要二次确认；管理记录不记谁让她做的，只有一个备注（`reason`）。
//!
//! 禁言照旧只做动作、不挡人（施工 O-48）。

use serde_json::{Value, json};

use miyu_im::toolset::{Call, NAME, Planned, clean, duration};

use super::acting::{Act, Undone, undone};
use crate::admin::{self, records};

mod kick;

/// 群管理在这个群没生效（插件关着、她不是管理员）答的那一句（施工 Q-1）。
pub(super) const ADMIN_OFF: &str = "admin-off";

/// 理由最多记几个字符（她给的一句话）。
pub(super) const REASON: usize = 200;

/// 禁言、踢人当场挡下的：插件在这个群没开着的答 `admin-off`，禁言秒数不在设置的最短、最长中间的答 `bad-seconds`（解禁不管）。别的
/// 动作、过了的是空的。
pub(super) fn checked(call: &Call, act: &Act) -> Option<Value> {
    if !matches!(act, Act::Mute { .. } | Act::Kick { .. }) {
        return None;
    }
    if !call.plugins.contains(&admin::ID) {
        return Some(call.answers.say(ADMIN_OFF, &[], true));
    }
    let (min, max) = admin::mute_range(&call.config);
    match act {
        Act::Mute { seconds, .. } if *seconds != 0 && !(min..=max).contains(seconds) => {
            Some(bad_seconds(call))
        }
        _ => None,
    }
}

/// 秒数写错、出了范围答的那一句：带设置的最短、最长（施工 Q-1）。
pub(super) fn bad_seconds(call: &Call) -> Value {
    let (min, max) = admin::mute_range(&call.config);
    let (min, max) = (duration(min), duration(max));
    call.answers
        .say("bad-seconds", &[("min", &min), ("max", &max)], true)
}

/// 禁言、踢人（`act` 已经认好目标）：当场挡下的当场答，别的交给 im 等着办（模块的说明）。
pub(super) fn moderated(call: Call, act: Act) -> Planned {
    if let Some(answer) = checked(&call, &act) {
        return Planned::Now(answer);
    }
    Planned::later(async move { done(&call, &act).await })
}

/// 办：她是管理员才做；群友叫的踢人先要确认；一个个做，成了的记进管理记录。
async fn done(call: &Call, act: &Act) -> Value {
    let Some(link) = &call.link else {
        return call.answers.say("unreachable", &[], true);
    };
    let (group, users) = match act {
        Act::Mute { group, users, .. } | Act::Kick { group, users, .. } => (*group, users),
        _ => return call.answers.say("unreachable", &[], true),
    };
    // 她在这个群里不是管理员、群主的，群管理不生效（施工 Q-1）：问不到身份的照问不到答。
    match admin::admin_here(link.as_ref(), group, call.bot).await {
        Ok(true) => {}
        Ok(false) => return call.answers.say(ADMIN_OFF, &[], true),
        Err(error) => return said(call, None, undone(error)),
    }
    let mut outcomes = Vec::new();
    for user in users {
        let outcome = match *act {
            Act::Mute { seconds, .. } => link.mute(group, *user, seconds).await.map_err(undone),
            Act::Kick { block, .. } => kick::kick(link.as_ref(), group, *user, block).await,
            _ => Err(Undone("unreachable", None)),
        };
        if outcome.is_ok() {
            recorded(call, act, *user).await;
        }
        outcomes.push((*user, outcome));
    }
    answered(call, act, &outcomes)
}

/// 一个人：这个群缓存着的名字，没有的写号。
fn who(call: &Call, user: i64) -> String {
    call.names
        .get(&user)
        .cloned()
        .unwrap_or_else(|| user.to_string())
}

/// 对 `user` 做 `act` 成了说的那一句。
fn success(call: &Call, act: &Act, user: i64) -> String {
    let who = who(call, user);
    match act {
        Act::Mute { seconds: 0, .. } => call.answers.text("unmuted", &[("who", &who)]),
        Act::Mute { seconds, .. } => call
            .answers
            .text("muted", &[("who", &who), ("duration", &duration(*seconds))]),
        Act::Kick { block: true, .. } => call.answers.text("kicked-blocked", &[("who", &who)]),
        _ => call.answers.text("kicked", &[("who", &who)]),
    }
}

/// 没做成说的那一句的字：带对谁（`user` 是空的：整次没做）、平台说的原因。
fn undone_text(call: &Call, user: Option<i64>, Undone(why, detail): &Undone) -> String {
    let who = user.map(|user| who(call, user)).unwrap_or_default();
    let detail = detail.as_deref().unwrap_or_default();
    call.answers
        .text(why, &[("who", who.as_str()), ("detail", detail)])
}

/// 没做成答的那一句。
fn said(call: &Call, user: Option<i64>, undone: Undone) -> Value {
    block(undone_text(call, user, &undone), true)
}

/// 一段字的工具结果。
fn block(text: String, error: bool) -> Value {
    json!({"blocks": [{"type": "text", "text": text}], "error": error})
}

/// 答的那一句（施工 I-17）：一个人的照原来的一句；几个人的头一行是成了几个，一人一行接在后面（成了的照成了的那一句，没成的那一句
/// 里没写是谁的前面写上）。都没成的算出错。
fn answered(call: &Call, act: &Act, outcomes: &[(i64, Result<(), Undone>)]) -> Value {
    if let [(user, outcome)] = outcomes {
        return match outcome {
            Ok(()) => block(success(call, act, *user), false),
            Err(undone) => block(undone_text(call, Some(*user), undone), true),
        };
    }
    let lines: Vec<String> = outcomes
        .iter()
        .map(|(user, outcome)| {
            let line = match outcome {
                Ok(()) => success(call, act, *user),
                Err(undone) => {
                    let who = who(call, *user);
                    let sentence = undone_text(call, Some(*user), undone);
                    match sentence.contains(&who) {
                        true => sentence,
                        false => format!("{who}: {sentence}"),
                    }
                }
            };
            line.trim_end().to_string()
        })
        .collect();
    let done = outcomes
        .iter()
        .filter(|(_, outcome)| outcome.is_ok())
        .count();
    let (done_count, total) = (done.to_string(), outcomes.len().to_string());
    let head = call
        .answers
        .text("batch", &[("done", &done_count), ("total", &total)]);
    // 模板换字会把字段里的换行转义掉：一行一个接在头一行后面（同查管理记录）。
    block(
        format!("{}\n{}\n", head.trim_end(), lines.join("\n")),
        done == 0,
    )
}

/// 对 `user` 做成了的禁言、解禁、踢人记进这个群的管理记录（施工 Q-2）：对谁、那时叫什么、她给的理由、谁让她做的（施工 I-17）；记录
/// 开关关了的不记。
async fn recorded(call: &Call, act: &Act, user: i64) {
    if !admin::recording(&call.config) {
        return;
    }
    let (group, action, seconds, block) = match *act {
        Act::Mute {
            group, seconds: 0, ..
        } => (group, records::Action::Unmute, None, false),
        Act::Mute { group, seconds, .. } => (group, records::Action::Mute, Some(seconds), false),
        Act::Kick { group, block, .. } => (group, records::Action::Kick, None, block),
        _ => return,
    };
    let named = |user: i64| call.names.get(&user).and_then(|name| clean(name, NAME));
    let record = records::Record {
        at: records::now(),
        action,
        user,
        name: named(user),
        by: records::By::Ai,
        seconds,
        reason: call.args["reason"]
            .as_str()
            .and_then(|reason| clean(reason.trim(), REASON)),
        block,
        verdict: None,
        comment: None,
    };
    records::append(call.store.clone(), group, record, admin::keep(&call.config)).await;
}
