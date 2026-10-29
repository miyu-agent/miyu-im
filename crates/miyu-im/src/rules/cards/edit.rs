//! 改一张卡（施工 O-58 下，「场所卡片的两页」的三个方法）：部分更新、加一张、删一张。纯逻辑：存盘、改白名单、推内容由调的一方做。
//!
//! - 部分更新（[`update`]）：`values` 只有改过的格，`null` 是恢复默认（删掉那一格）。每一格先单独照卡片的样子读一遍：认不得的、
//!   类型不对的报在那一格上（`value`）；都读得出的合进去，再拼成规则交群聊内核读，有问题的报在对应的格上（规则的键换回卡片的格）。
//! - 那张卡不在的（群卡、私聊卡）交回 [`Edit::Missing`]。
//! - 卡上的 `rules`（施工 I-7 中一）照项部分更新：`values` 里的 `rules` 只写改了的键，`null` 是去掉那一个；键写法不对的报在
//!   `rules.<键>` 上（`unknown_key`），去光了的整格去掉。规则写错报在那个键上的，照卡上 `rules` 里有没有它报 `rules.<键>`。

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use super::lines::key_ok;
use super::{Card, Cards, GroupCard, GroupSet, Others, PrivateCard, PrivateSet, platform_ok};
use crate::chat::Id;

/// 改完的样子。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// 改好了的全部卡片：调的一方存盘、换上。
    Done(Box<Cards>),
    /// 有问题，什么都不改：哪一格、原因码（群聊内核的，或者读不出的 `value`）。
    Problems(Vec<(String, String)>),
    /// 那张卡不在。
    Missing,
}

/// 照页面交来的 `{"kind", "platform"?, "id"?}` 认是哪一张卡（施工 I-80：没写平台的照 `fallback`，调的一方照挂着的平台定；
/// 定不了的是空的）。平台名不合写法的、号不合写法的是空的。
pub fn card_of(value: &Value, fallback: Option<&str>) -> Option<Card> {
    let id = || serde_json::from_value::<Id>(value["id"].clone()).ok();
    let platform = match &value["platform"] {
        Value::Null if value["kind"] == "defaults" => String::new(),
        Value::Null => fallback?.to_string(),
        named => named.as_str().filter(|name| platform_ok(name))?.to_string(),
    };
    match value["kind"].as_str()? {
        "defaults" => Some(Card::Defaults),
        "other_groups" => Some(Card::OtherGroups(platform)),
        "other_privates" => Some(Card::OtherPrivates(platform)),
        "group" => Some(Card::Group(platform, id()?)),
        "private" => Some(Card::Private(platform, id()?)),
        _ => None,
    }
}

/// 把 `values` 合进 `card` 那一张（部分更新），查过的交回。
pub fn update(cards: &Cards, card: Card, values: &Map<String, Value>) -> Edit {
    let mut next = cards.clone();
    let merged = match &card {
        Card::Defaults => merge(&mut next.defaults, values),
        Card::OtherGroups(platform) => merge(next.other_groups_mut(platform), values),
        Card::OtherPrivates(platform) => merge(next.other_privates_mut(platform), values),
        Card::Group(platform, id) => match group_mut(&mut next, platform, id) {
            Some(one) => merge(&mut one.set, values),
            None => return Edit::Missing,
        },
        Card::Private(platform, id) => match private_mut(&mut next, platform, id) {
            Some(one) => merge(&mut one.set, values),
            None => return Edit::Missing,
        },
    };
    if let Err(problems) = merged {
        return Edit::Problems(problems);
    }
    next.others.retain(|_, others| *others != Others::default());
    let written = written_rules(&next, &card);
    let problems: Vec<(String, String)> = next
        .problems()
        .into_iter()
        .filter(|problem| Card::parse(&problem.file).as_ref() == Some(&card))
        .map(|problem| {
            (
                problem
                    .key
                    .as_deref()
                    .map(|key| match written.iter().any(|one| one == key) {
                        true => format!("rules.{key}"),
                        false => field_of(key).to_string(),
                    })
                    .unwrap_or_default(),
                problem.code.as_str().to_string(),
            )
        })
        .collect();
    if problems.is_empty() {
        Edit::Done(Box::new(next))
    } else {
        Edit::Problems(problems)
    }
}

/// 平台 `platform` 上群号 `id` 的那张卡。
fn group_mut<'a>(cards: &'a mut Cards, platform: &str, id: &Id) -> Option<&'a mut GroupCard> {
    cards
        .groups
        .iter_mut()
        .find(|one| one.platform == platform && one.id == *id)
}

/// 平台 `platform` 上对方的号是 `id` 的那张卡。
fn private_mut<'a>(cards: &'a mut Cards, platform: &str, id: &Id) -> Option<&'a mut PrivateCard> {
    cards
        .privates
        .iter_mut()
        .find(|one| one.platform == platform && one.id == *id)
}

/// 加一张空卡：已经有的照旧（交回原样）。「其他群」「其他私聊」本来就有。
pub fn add(cards: &Cards, card: Card) -> Cards {
    let mut next = cards.clone();
    match card {
        Card::Defaults | Card::OtherGroups(_) | Card::OtherPrivates(_) => {}
        Card::Group(platform, id) => {
            if group_mut(&mut next, &platform, &id).is_none() {
                next.groups.push(GroupCard {
                    platform,
                    id,
                    set: GroupSet::default(),
                });
            }
        }
        Card::Private(platform, id) => {
            if private_mut(&mut next, &platform, &id).is_none() {
                next.privates.push(PrivateCard {
                    platform,
                    id,
                    set: PrivateSet::default(),
                });
            }
        }
    }
    next
}

/// 删一张：会话默认值、「其他群」「其他私聊」删不掉（交回空的），没有的照旧。
pub fn remove(cards: &Cards, card: Card) -> Option<Cards> {
    let mut next = cards.clone();
    match card {
        Card::Defaults | Card::OtherGroups(_) | Card::OtherPrivates(_) => return None,
        Card::Group(platform, id) => next
            .groups
            .retain(|one| one.platform != platform || one.id != id),
        Card::Private(platform, id) => next
            .privates
            .retain(|one| one.platform != platform || one.id != id),
    }
    Some(next)
}

/// 卡 `card` 上 `rules` 里写着的键。
fn written_rules(cards: &Cards, card: &Card) -> Vec<String> {
    let set = match card {
        Card::Defaults => serde_json::to_value(&cards.defaults),
        Card::OtherGroups(platform) => serde_json::to_value(cards.other_groups_of(platform)),
        Card::OtherPrivates(platform) => serde_json::to_value(cards.other_privates_of(platform)),
        Card::Group(platform, id) => serde_json::to_value(
            cards
                .groups
                .iter()
                .find(|one| one.platform == *platform && one.id == *id)
                .map(|one| &one.set),
        ),
        Card::Private(platform, id) => serde_json::to_value(
            cards
                .privates
                .iter()
                .find(|one| one.platform == *platform && one.id == *id)
                .map(|one| &one.set),
        ),
    };
    set.ok()
        .and_then(|set| {
            set["rules"]
                .as_object()
                .map(|rules| rules.keys().cloned().collect())
        })
        .unwrap_or_default()
}

/// `values` 里的 `rules` 照项合进 `set` 原来的那一份（模块的说明）：交回合好的 `values`；键写法不对的交回哪几个。
fn merge_rules<T: Serialize>(
    set: &T,
    values: &Map<String, Value>,
) -> Result<Map<String, Value>, Vec<(String, String)>> {
    let Some(changes) = values.get("rules") else {
        return Ok(values.clone());
    };
    let Some(changes) = changes.as_object() else {
        return Err(vec![("rules".to_string(), "value".to_string())]);
    };
    let bad: Vec<(String, String)> = changes
        .keys()
        .filter(|key| !key_ok(key))
        .map(|key| (format!("rules.{key}"), "unknown_key".to_string()))
        .collect();
    if !bad.is_empty() {
        return Err(bad);
    }
    let mut rules = serde_json::to_value(set)
        .ok()
        .and_then(|set| set["rules"].as_object().cloned())
        .unwrap_or_default();
    for (key, value) in changes {
        if value.is_null() {
            rules.remove(key);
        } else {
            rules.insert(key.clone(), value.clone());
        }
    }
    let mut merged = values.clone();
    merged.insert(
        "rules".to_string(),
        if rules.is_empty() {
            Value::Null
        } else {
            Value::Object(rules)
        },
    );
    Ok(merged)
}

/// 每一格单独照 `T` 读一遍，都读得出的合进 `set`；读不出的交回哪几格。`rules` 照项合（[`merge_rules`]）。
fn merge<T: Serialize + DeserializeOwned>(
    set: &mut T,
    values: &Map<String, Value>,
) -> Result<(), Vec<(String, String)>> {
    let values = &merge_rules(set, values)?;
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
