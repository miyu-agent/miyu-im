//! 同步群里别的管理员做的（施工 Q-3，设计 02 第一节第 2、3 条）：NapCat 推来的 `group_ban`（禁言、解禁别人）、`group_decrease`
//! 的 `kick`（踢人），做的人不是她（她自己做的工具那一头记过了）、对的人也不是她（她被禁言 im 记过事实，O-50），就记进管理记录、
//! 给那个群的会话记一句 `moderated`（项目主人 10-19：别的管理员做的也要让 AI 知道）。群管理在这个群生效才管：插件开着、她是管理员或
//! 群主（第四节第 2 条）。别的管理员解禁的，「同步解禁」关了的不管。
//!
//! 一条连接另起一个任务照先后办（[`start`]）：要问 NapCat 两个人的名字和她的身份，回应在读这条连接的那一头读到，不能在那一头等。
//! 踢走的人问不到名字，写号。
//!
//! 她自己踢的照旧不在这里记（工具那一头记过了），只记进「刚踢走的」（[`Removals`]，施工 I-17，同旧版的 `group_decrease` 台账）：
//! NapCat 踢成了也可能回失败，踢人的工具照它核实人走没走。

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::mpsc;

use miyu_im::chat::{Id, Link, To};
use miyu_im::driver::Gate;
use miyu_im::toolset::{NAME, clean};

use super::records::{self, Action, By, Record};
use crate::admin::{self, ID};

/// 给 AI 记的那一句的种类（当标签名）。
const KIND: &str = "moderated";

/// 群里别的管理员做的一件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Moderated {
    /// 收到通知的机器人的号。
    pub bot: Id,
    /// 群号。
    pub group: Id,
    /// 对谁。
    pub user: Id,
    /// 谁做的。
    pub operator: Id,
    /// 禁言几秒；解禁、踢人的是空的。
    pub seconds: Option<u64>,
    /// 做了什么。
    pub action: Action,
}

/// 读一帧通知 `frame`（纯逻辑）：别的管理员禁言、解禁、踢别人的才是；她做的、对她做的、全员禁言、不知道谁做的、别的不是。
pub(crate) fn read(frame: &Value) -> Option<Moderated> {
    let id = |value: &Value| value.as_i64().filter(|number| *number > 0).map(Id::from);
    let (bot, group, user, operator) = (
        id(&frame["self_id"])?,
        id(&frame["group_id"])?,
        id(&frame["user_id"])?,
        id(&frame["operator_id"])?,
    );
    if user == bot || operator == bot {
        return None;
    }
    let (action, seconds) = match (frame["notice_type"].as_str()?, frame["sub_type"].as_str()?) {
        ("group_ban", "ban") => match frame["duration"].as_u64()? {
            0 => (Action::Unmute, None),
            seconds => (Action::Mute, Some(seconds)),
        },
        ("group_ban", "lift_ban") => (Action::Unmute, None),
        ("group_decrease", "kick") => (Action::Kick, None),
        _ => return None,
    };
    Some(Moderated {
        bot,
        group,
        user,
        operator,
        seconds,
        action,
    })
}

/// 刚踢走的记多久（同旧版：踢成了一两秒内必到）。
const REMOVED_KEPT: Duration = Duration::from_secs(60);

/// 她自己刚踢走的（施工 I-17）：群号、号 → 收到通知的时刻。
#[derive(Debug, Default)]
pub(crate) struct Removals(HashMap<(Id, Id), Instant>);

impl Removals {
    /// 记下群 `group` 里 `user` 在 `now` 被她踢走了；过了 [`REMOVED_KEPT`] 的顺手拿掉。
    pub(crate) fn note(&mut self, (group, user): (Id, Id), now: Instant) {
        self.0
            .retain(|_, at| now.duration_since(*at) < REMOVED_KEPT);
        self.0.insert((group, user), now);
    }

    /// 群 `group` 里 `user` 是不是 `now` 以前 [`REMOVED_KEPT`] 以内被她踢走的。
    pub(crate) fn lately(&self, (group, user): (Id, Id), now: Instant) -> bool {
        self.0
            .get(&(group, user))
            .is_some_and(|at| now.duration_since(*at) < REMOVED_KEPT)
    }
}

/// 这个进程里她刚踢走的。
static REMOVED: LazyLock<Mutex<Removals>> = LazyLock::new(Mutex::default);

/// 一帧通知 `frame` 是不是她自己把人踢出群（`group_decrease` 的 `kick`，做的人是她）：是的交回群号、号（纯逻辑）。
pub(crate) fn removed_by_her(frame: &Value) -> Option<(Id, Id)> {
    let id = |value: &Value| value.as_i64().filter(|number| *number > 0).map(Id::from);
    let (bot, operator) = (id(&frame["self_id"])?, id(&frame["operator_id"])?);
    let kicked = frame["notice_type"] == "group_decrease" && frame["sub_type"] == "kick";
    (kicked && operator == bot).then_some((id(&frame["group_id"])?, id(&frame["user_id"])?))
}

/// 群 `group` 里 `user` 是不是刚被她踢走了（施工 I-17：踢人的工具核实用）。
pub(crate) fn removed_lately(group: Id, user: Id) -> bool {
    REMOVED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .lately((group, user), Instant::now())
}

/// 一条连接上照先后记的那一头（模块的说明）：另起一个任务一件件办，经那条连接 `link` 问名字、身份；连接断了（发的一头放下了）就停。
pub(crate) fn start(gate: &Arc<Gate>, link: &Arc<dyn Link>) -> mpsc::UnboundedSender<Moderated> {
    let (sender, mut received) = mpsc::unbounded_channel();
    let (gate, link) = (Arc::clone(gate), Arc::clone(link));
    tokio::spawn(async move {
        while let Some(moderated) = received.recv().await {
            synced(&gate, link.as_ref(), moderated).await;
        }
    });
    sender
}

/// 一帧通知 `frame` 是别的管理员做的就交给那条连接照先后记的那一头 `syncing`；她自己踢走的记进「刚踢走的」（施工 I-17）。
pub(crate) fn seen(syncing: &mpsc::UnboundedSender<Moderated>, frame: &Value) {
    if let Some(removed) = removed_by_her(frame) {
        REMOVED
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .note(removed, Instant::now());
    }
    if let Some(moderated) = read(frame)
        && syncing.send(moderated).is_err()
    {
        tracing::debug!(target: miyu_im::TARGET, "moderation sync stopped");
    }
}

/// 记一件：群管理在这个群生效才记；进管理记录（记录开关开着的）、给会话记一句。
async fn synced(gate: &Gate, link: &dyn Link, moderated: Moderated) {
    let Moderated {
        bot,
        group,
        user,
        operator,
        seconds,
        action,
    } = moderated;
    let to = To::Group(group.clone());
    if !gate.active(ID, &bot, to.clone()) {
        return;
    }
    let config = gate.config().borrow().clone();
    if action == Action::Unmute && !admin::switched(&config, admin::SYNC_UNMUTE) {
        return;
    }
    if !matches!(admin::admin_here(link, &group, &bot).await, Ok(true)) {
        return;
    }
    let name = |member: Option<String>| member.and_then(|name| clean(&name, NAME));
    let target = name(
        link.member(&group, &user)
            .await
            .ok()
            .and_then(|one| one.name),
    );
    let by = name(
        link.member(&group, &operator)
            .await
            .ok()
            .and_then(|one| one.name),
    );
    if admin::recording(&config) {
        let record = Record {
            at: records::now(),
            action,
            user: user.clone(),
            name: target.clone(),
            by: By::Admin {
                user: operator.clone(),
                name: by.clone(),
            },
            seconds,
            reason: None,
            block: false,
            verdict: None,
            comment: None,
        };
        records::append(
            gate.store().to_path_buf(),
            group.clone(),
            record,
            admin::keep(&config),
        )
        .await;
    }
    let who = target.unwrap_or_else(|| user.to_string());
    let by = by.unwrap_or_else(|| operator.to_string());
    let noted = match (action, seconds) {
        (Action::Mute, Some(seconds)) => {
            let duration = spoken(seconds);
            let fields = [("who", who.as_str()), ("by", &by), ("duration", &duration)];
            gate.note((bot, to), KIND, "muted-by", &fields).await
        }
        (Action::Kick, _) => {
            let fields = [("who", who.as_str()), ("by", &by)];
            gate.note((bot, to), KIND, "kicked-by", &fields).await
        }
        _ => {
            let fields = [("who", who.as_str()), ("by", &by)];
            gate.note((bot, to), KIND, "unmuted-by", &fields).await
        }
    };
    if !noted {
        tracing::debug!(target: miyu_im::TARGET, group = %group, "moderation not noted, stopping");
    }
}

/// 多久，照给 AI 看的英文写（纯逻辑，设计 02 第一节第 3 条：换成分钟、小时、天）：天、小时、分钟各一段，0 的不写，不到一分钟的
/// 照一分钟。
pub(crate) fn spoken(seconds: u64) -> String {
    let minutes = seconds.div_ceil(60).max(1);
    let parts = [
        (minutes / 1440, "day"),
        (minutes % 1440 / 60, "hour"),
        (minutes % 60, "minute"),
    ];
    parts
        .iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, unit)| match count {
            1 => format!("1 {unit}"),
            count => format!("{count} {unit}s"),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests;
