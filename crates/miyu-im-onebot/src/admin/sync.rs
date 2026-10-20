//! 同步群里别的管理员做的（施工 Q-3，设计 02 第一节第 2、3 条）：NapCat 推来的 `group_ban`（禁言、解禁别人）、`group_decrease`
//! 的 `kick`（踢人），做的人不是她（她自己做的工具那一头记过了）、对的人也不是她（她被禁言 im 记过事实，O-50），就记进管理记录、
//! 给那个群的会话记一句 `moderated`（项目主人 10-19：别的管理员做的也要让 AI 知道）。群管理在这个群生效才管：插件开着、她是管理员或
//! 群主（第四节第 2 条）。别的管理员解禁的，「同步解禁」关了的不管。
//!
//! 一条连接另起一个任务照先后办（[`start`]）：要问 NapCat 两个人的名字和她的身份，回应在读这条连接的那一头读到，不能在那一头等。
//! 踢走的人问不到名字，写号。

use std::sync::Arc;

use serde_json::Value;
use tokio::sync::mpsc;

use miyu_im::chat::{Link, To};
use miyu_im::driver::Gate;
use miyu_im::toolset::{NAME, clean};

use super::records::{self, Action, By, Record};
use crate::admin::{self, ID};

/// 给 AI 记的那一句的种类（当标签名）。
const KIND: &str = "moderated";

/// 群里别的管理员做的一件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Moderated {
    /// 收到通知的机器人的号。
    pub bot: i64,
    /// 群号。
    pub group: i64,
    /// 对谁。
    pub user: i64,
    /// 谁做的。
    pub operator: i64,
    /// 禁言几秒；解禁、踢人的是空的。
    pub seconds: Option<u64>,
    /// 做了什么。
    pub action: Action,
}

/// 读一帧通知 `frame`（纯逻辑）：别的管理员禁言、解禁、踢别人的才是；她做的、对她做的、全员禁言、不知道谁做的、别的不是。
pub(crate) fn read(frame: &Value) -> Option<Moderated> {
    let number = |value: &Value| value.as_i64().filter(|number| *number > 0);
    let (bot, group, user, operator) = (
        number(&frame["self_id"])?,
        number(&frame["group_id"])?,
        number(&frame["user_id"])?,
        number(&frame["operator_id"])?,
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

/// 一帧通知 `frame` 是别的管理员做的就交给那条连接照先后记的那一头 `syncing`。
pub(crate) fn seen(syncing: &mpsc::UnboundedSender<Moderated>, frame: &Value) {
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
    let to = To::Group(group);
    if !gate.active(ID, to) {
        return;
    }
    let config = gate.config().borrow().clone();
    if action == Action::Unmute && !admin::switched(&config, admin::SYNC_UNMUTE) {
        return;
    }
    if !matches!(admin::admin_here(link, group, bot).await, Ok(true)) {
        return;
    }
    let name = |member: Option<String>| member.and_then(|name| clean(&name, NAME));
    let target = name(link.member(group, user).await.ok().and_then(|one| one.name));
    let by = name(
        link.member(group, operator)
            .await
            .ok()
            .and_then(|one| one.name),
    );
    if admin::recording(&config) {
        let record = Record {
            at: records::now(),
            action,
            user,
            name: target.clone(),
            by: By::Admin {
                user: operator,
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
            group,
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
        tracing::debug!(target: miyu_im::TARGET, group, "moderation not noted, stopping");
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
