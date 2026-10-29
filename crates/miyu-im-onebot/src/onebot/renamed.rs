//! 群名片改了（施工 I-38）：NapCat 推来的 `group_card` 通知（`card_new`、`card_old`）认成 [`Event::Renamed`]，im 拿它更新群成员
//! 的名字缓存；改的是她自己的，下一条场所消息的 `my_name` 就带新的。`card_new` 空白的是清掉了（显示回昵称）。

use serde_json::Value;

use miyu_im::chat::Event;

use super::id;

/// 认一帧通知；不是 `group_card` 的、号读不出的是空的。
pub(super) fn renamed(frame: &Value) -> Option<Event> {
    if frame["notice_type"] != "group_card" {
        return None;
    }
    let card = frame["card_new"]
        .as_str()
        .filter(|card| !card.trim().is_empty())
        .map(str::to_string);
    Some(Event::Renamed {
        bot: id(&frame["self_id"])?,
        group: id(&frame["group_id"])?,
        user: id(&frame["user_id"])?,
        card,
    })
}

#[cfg(test)]
mod tests;
