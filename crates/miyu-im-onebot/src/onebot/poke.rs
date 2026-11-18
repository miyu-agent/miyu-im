//! 戳一戳（QQ 原来叫拍一拍；施工 I-248，2026-11-17 项目主人定：私聊里拍她开一轮；群里拍她当作冲她来的一条，照群里的规则判回不回；拍别人的只记进
//! 聊天记录）：NapCat 推来的 `notice`，`notice_type` 是 `notify`、`sub_type` 是 `poke`；`user_id` 是拍的人，`target_id` 是被拍的人，
//! 群里的带 `group_id`。
//!
//! - 拍她的（`target_id` 是收到它的机器人号）：交成拍的人发来的一条（[`Posted::poke`]），私聊的是 [`Event::Private`]，群里的是
//!   [`Event::Group`]。没有字和段，名字照群成员的缓存补（群里），编号照 [`miyu_im::chat::poke_number`] 编：平台不给编号，
//!   编的是负的、比 32 位整数小，撞不上 QQ 的消息编号，同一毫秒的也不重（施工 AUD-3）。
//! - 群里拍别人的：一件 [`GroupChange::Poked`]，只记进聊天记录。私聊里拍别人（拍自己）的、她自己拍的、号读不出的不是。
//! - 拍的人自己设的后缀（「拍了拍你的小脑袋」里的「的小脑袋」，[`Poke::suffix`]）：NapCat 的 `raw_info` 是那条灰条拆成的几段，`type`
//!   是 `nor` 的是字，第一段是动作（「戳了戳」，QQ 改名以前是「拍了拍」，哪个都一样当动作），后面的接起来是后缀（[`suffix`]）。NapCat 的源码已经不公开、文档没写，这是照记得的
//!   样子认的，认不出的不写；拍她的那一次把整份 `raw_info` 记一行运行日志（`poke raw`），对得上对不上照真机的看。

use serde_json::Value;

use miyu_im::TARGET;
use miyu_im::chat::{Event, GroupChange, Happened, Poke, Posted, Segments};

use super::{id, number};

/// 认一条拍一拍（见模块开头）；不是的交回空的。
pub fn poked(frame: &Value) -> Option<Event> {
    if frame["notice_type"] != "notify" || frame["sub_type"] != "poke" {
        return None;
    }
    let (bot, user, target) = (
        id(&frame["self_id"])?,
        id(&frame["user_id"])?,
        id(&frame["target_id"])?,
    );
    if user == bot {
        return None;
    }
    let group = match &frame["group_id"] {
        Value::Null => None,
        value => Some(id(value)?),
    };
    if target != bot {
        return Some(Event::Happened(Happened {
            bot,
            group: group?,
            change: GroupChange::Poked,
            user: target,
            operator: Some(user),
            seconds: None,
        }));
    }
    tracing::info!(target: TARGET, user = %user, raw = %frame["raw_info"], "poke raw");
    let posted = Posted {
        bot,
        user,
        message_id: miyu_im::chat::poke_number(),
        time: number(&frame["time"]).unwrap_or(0),
        name: None,
        rank: None,
        text: String::new(),
        segments: Segments::default(),
        poke: Some(Poke {
            suffix: suffix(&frame["raw_info"]),
        }),
    };
    Some(match group {
        None => Event::Private(posted),
        Some(group) => Event::Group { group, posted },
    })
}

/// `raw_info` 里动作后面的字（见模块开头）：`type` 是 `nor` 的几段，去掉第一段（动作），剩下的接起来、去掉首尾空白不空的才有。
fn suffix(raw: &Value) -> Option<String> {
    let words: Vec<&str> = raw
        .as_array()?
        .iter()
        .filter(|part| part["type"] == "nor")
        .filter_map(|part| part["txt"].as_str())
        .collect();
    let rest = words.get(1..)?.concat();
    let rest = rest.trim();
    (!rest.is_empty()).then(|| rest.to_string())
}

#[cfg(test)]
mod tests;
