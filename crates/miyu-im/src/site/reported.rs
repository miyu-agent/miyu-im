//! 连接卡并进驱动报的（施工 I-183 下中，设计 07 第八节）：令牌表的驱动（Telegram）在状态里报每个连接（`connections` 里带 `index` 的那
//! 几项：号、用户名、名字、隐私模式、毛病），宿主连接卡上原来只有连着、断开了的号。这里把用户名、隐私模式、毛病并进同一个号的卡；还没认出号、
//! 号从没连上过的（令牌一开始就失效、网络不通）另画一张，编号是 `<平台>:#<第几个>`，带 `index` 和毛病，页面照它说为什么。端口表的驱动报的
//! 连接没有 `index`，不碰。

use serde_json::{Map, Value, json};

/// 几张连接卡 `cards`（平台 `platform`、能力 `caps`）并进驱动报的状态 `status`。
pub(super) fn merge(
    cards: &mut Vec<Value>,
    status: &Map<String, Value>,
    (platform, caps): (&str, &[&str]),
) {
    let reported = status
        .get("connections")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    for one in reported.filter(|one| one.get("index").is_some()) {
        let bot = one["bot"].as_str();
        let card = bot.and_then(|bot| {
            cards
                .iter_mut()
                .find(|card| card["bot"].as_str() == Some(bot))
        });
        match card {
            Some(Value::Object(card)) => copy(one, card),
            _ => {
                let mut card = Map::new();
                let id = bot.map_or_else(
                    || format!("{platform}:#{}", one["index"]),
                    |bot| format!("{platform}:{bot}"),
                );
                card.insert("id".into(), json!(id));
                card.insert("platform".into(), json!(platform));
                card.insert("bot".into(), json!(bot.unwrap_or_default()));
                card.insert("connected".into(), json!(false));
                card.insert("caps".into(), json!(caps));
                if let Some(name) = one["name"].as_str().filter(|name| !name.is_empty()) {
                    card.insert("name".into(), json!(name));
                }
                copy(one, &mut card);
                cards.push(Value::Object(card));
            }
        }
    }
}

/// 驱动报的这一个连接 `one` 里页面要的几格抄进卡 `card`：第几个、用户名、隐私模式开着的当毛病 `privacy`（连着时），别的毛病照报的。
fn copy(one: &Value, card: &mut Map<String, Value>) {
    card.insert("index".into(), one["index"].clone());
    if let Some(username) = one["username"]
        .as_str()
        .filter(|username| !username.is_empty())
    {
        card.insert("username".into(), json!(username));
    }
    let connected = card
        .get("connected")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let trouble = match one["trouble"].as_str() {
        Some(trouble) => Some(trouble),
        None if connected && one["privacy"] == true => Some("privacy"),
        None => None,
    };
    if let Some(trouble) = trouble {
        card.insert("trouble".into(), json!(trouble));
    }
}

#[cfg(test)]
mod tests;
