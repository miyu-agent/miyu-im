//! 改一张卡（施工 O-58 下，「场所卡片的两页」的三个方法）：部分更新、加一张、删一张。纯逻辑：存盘、改白名单、推内容由调的一方做。
//!
//! - 部分更新（[`update`]）：`values` 只有改过的格，`null` 是恢复默认（删掉那一格）。每一格先单独照卡片的样子读一遍：认不得的、
//!   类型不对的报在那一格上（`value`）；都读得出的合进去，再拼成规则交群聊内核读，有问题的报在对应的格上（规则的键换回卡片的格）。
//! - 那张卡不在的（群卡、私聊卡）交回 [`Edit::Missing`]。

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use super::{Card, Cards, GroupCard, GroupSet, PrivateCard, PrivateSet};

/// 改完的样子。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// 改好了的全部卡片：调的一方存盘、换上。
    Done(Cards),
    /// 有问题，什么都不改：哪一格、原因码（群聊内核的，或者读不出的 `value`）。
    Problems(Vec<(String, String)>),
    /// 那张卡不在。
    Missing,
}

/// 照页面交来的 `{"kind", "id"?}` 认是哪一张卡。
pub fn card_of(value: &Value) -> Option<Card> {
    let id = || value["id"].as_u64();
    match value["kind"].as_str()? {
        "other_groups" => Some(Card::OtherGroups),
        "group" => Some(Card::Group(id()?)),
        "private" => Some(Card::Private(id()?)),
        _ => None,
    }
}

/// 把 `values` 合进 `card` 那一张（部分更新），查过的交回。
pub fn update(cards: &Cards, card: Card, values: &Map<String, Value>) -> Edit {
    let mut next = cards.clone();
    let merged = match card {
        Card::OtherGroups => merge(&mut next.other_groups, values),
        Card::Group(id) => match next.groups.iter_mut().find(|one| one.id == id) {
            Some(one) => merge(&mut one.set, values),
            None => return Edit::Missing,
        },
        Card::Private(id) => match next.privates.iter_mut().find(|one| one.id == id) {
            Some(one) => merge(&mut one.set, values),
            None => return Edit::Missing,
        },
    };
    if let Err(problems) = merged {
        return Edit::Problems(problems);
    }
    let problems: Vec<(String, String)> = next
        .problems()
        .into_iter()
        .filter(|problem| Card::parse(&problem.file) == Some(card))
        .map(|problem| {
            (
                problem
                    .key
                    .as_deref()
                    .map(field_of)
                    .unwrap_or_default()
                    .to_string(),
                problem.code.as_str().to_string(),
            )
        })
        .collect();
    if problems.is_empty() {
        Edit::Done(next)
    } else {
        Edit::Problems(problems)
    }
}

/// 加一张空卡：已经有的照旧（交回原样）。「其他群」本来就有。
pub fn add(cards: &Cards, card: Card) -> Cards {
    let mut next = cards.clone();
    match card {
        Card::OtherGroups => {}
        Card::Group(id) if !next.groups.iter().any(|one| one.id == id) => {
            next.groups.push(GroupCard {
                id,
                set: GroupSet::default(),
            })
        }
        Card::Private(id) if !next.privates.iter().any(|one| one.id == id) => {
            next.privates.push(PrivateCard {
                id,
                set: PrivateSet::default(),
            });
        }
        Card::Group(_) | Card::Private(_) => {}
    }
    next
}

/// 删一张：「其他群」删不掉（交回空的），没有的照旧。
pub fn remove(cards: &Cards, card: Card) -> Option<Cards> {
    let mut next = cards.clone();
    match card {
        Card::OtherGroups => return None,
        Card::Group(id) => next.groups.retain(|one| one.id != id),
        Card::Private(id) => next.privates.retain(|one| one.id != id),
    }
    Some(next)
}

/// 每一格单独照 `T` 读一遍，都读得出的合进 `set`；读不出的交回哪几格。
fn merge<T: Serialize + DeserializeOwned>(
    set: &mut T,
    values: &Map<String, Value>,
) -> Result<(), Vec<(String, String)>> {
    let bad: Vec<(String, String)> = values
        .iter()
        .filter_map(|(key, value)| {
            if !known::<T>(key) {
                return Some((key.clone(), "unknown_key".to_string()));
            }
            let one = Value::Object(Map::from_iter([(key.clone(), value.clone())]));
            serde_json::from_value::<T>(one)
                .is_err()
                .then(|| (key.clone(), "value".to_string()))
        })
        .collect();
    if !bad.is_empty() {
        return Err(bad);
    }
    let mut object = match serde_json::to_value(&*set) {
        Ok(Value::Object(object)) => object,
        _ => Map::new(),
    };
    for (key, value) in values {
        if value.is_null() {
            object.remove(key);
        } else {
            object.insert(key.clone(), value.clone());
        }
    }
    // 每一格都单独读过：合起来的照样读得出。
    *set = serde_json::from_value(Value::Object(object)).map_err(|_| Vec::new())?;
    Ok(())
}

/// `T` 认不认 `key` 这一格：照一个只有它、值是 `null` 的对象读（每一格都是可选的，`null` 读成没写）。
fn known<T: DeserializeOwned>(key: &str) -> bool {
    serde_json::from_value::<T>(Value::Object(Map::from_iter([(
        key.to_string(),
        Value::Null,
    )])))
    .is_ok()
}

/// 群聊内核报的规则的键换回卡片的格：`allow` 是 `on`，插嘴拼成的两项是 `chatty`，出站表里的去掉 `outbound.`。
fn field_of(rule: &str) -> &str {
    match rule {
        "allow" => "on",
        "discipline" | "chatty.probability" => "chatty",
        other => other.strip_prefix("outbound.").unwrap_or(other),
    }
}

#[cfg(test)]
mod tests;
