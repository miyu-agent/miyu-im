//! 后台页的场所卡片（施工 O-58 上，`onebot.md` 第一条「场所卡片」）：`cards.get`、`cards.save` 两个方法，读的一头照推送交到这里，
//! 卡片、场所规则、连接都在这一头。
//!
//! - `cards.get`：卡片、每一格的默认值（卡片下面几层套出来的，`rules::card_defaults`）、白名单成员的号当场交；她在的群、好友
//!   （NapCat 的 `get_group_list`、`get_friend_list`，连着的号都问）和人格的名字（核心的 `persona.list`）另起任务问，问不到的
//!   照空的交，页面写号。
//! - `cards.save {cards}`：读不成卡片的样子拒 `bad_params`；拼成规则有问题的不存，答 `{saved: false, problems}`，问题一条条
//!   交回（哪一张卡、哪一格、原因码、原文；不拒：核心转方法的错误只带原因，带不过 `data`）；写不进盘的拒 `not_saved`；存好了
//!   答 `{saved: true}`、当场换上，下一条消息就照新的套。

use std::sync::Arc;

use serde_json::{Map, Value, json};

use super::Route;
use crate::TARGET;
use crate::core::Answerer;
use crate::core::caller::Caller;
use crate::core::methods::{CARDS_GET, CARDS_SAVE};
use crate::core::reason;
use crate::listen::bots::Bots;
use crate::onebot::venue;
use crate::rules::{Card, Cards, card_defaults, save_cards};
use miyu_chat::VenueKind;

impl Route {
    /// 核心转来的一次 `method.call`（`pushed`）是不是场所卡片的：是的照方法答，交回真。附加提示词不在这里重放：每一条消息进来
    /// 都照这一刻的规则放（`titled`），回合也只在来了消息以后开。
    pub(super) fn cards_called(&mut self, pushed: &Value) -> bool {
        let id = pushed["id"].clone();
        match pushed["params"]["method"].as_str() {
            Some(CARDS_GET) => self.cards_get(id),
            Some(CARDS_SAVE) => self.cards_save(&id, &pushed["params"]["params"]),
            _ => return false,
        }
        true
    }

    /// `cards.get`：手头的当场拼好，群、好友、人格另起任务问齐了再答。
    fn cards_get(&mut self, id: Value) {
        let cards = self.rules.cards().clone();
        let mut groups = Map::new();
        for card in &cards.groups {
            groups.insert(card.id.to_string(), self.group_defaults(card.id));
        }
        let mut privates = Map::new();
        for card in &cards.privates {
            privates.insert(card.id.to_string(), self.private_defaults(card.id));
        }
        let defaults = json!({
            // 「其他群」：照一个不在任何名单里的群号套（群号 0 不是真群）。
            "other_groups": self.group_defaults(0),
            "groups": groups,
            "privates": privates,
        });
        let whitelist: Vec<u64> = self
            .whitelist
            .iter()
            .filter_map(|identity| identity.strip_prefix("qq:")?.parse().ok())
            .collect();
        let known = json!({"cards": cards, "defaults": defaults, "whitelist": whitelist});
        let (bots, caller, answerer) = (
            Arc::clone(&self.bots),
            self.core.caller(),
            self.core.answerer(),
        );
        self.chores.spawn(listed(id, known, bots, caller, answerer));
    }

    /// 群号是 `group` 的群卡每一格的默认值。
    pub(super) fn group_defaults(&self, group: u64) -> Value {
        match i64::try_from(group)
            .ok()
            .and_then(|number| venue(VenueKind::Group, number).ok())
        {
            Some(venue) => card_defaults::group(&self.rules.beneath(&venue)),
            None => Value::Null,
        }
    }

    /// 对方的号是 `user` 的私聊卡每一格的默认值。
    pub(super) fn private_defaults(&self, user: u64) -> Value {
        match i64::try_from(user)
            .ok()
            .and_then(|number| venue(VenueKind::Private, number).ok())
        {
            Some(venue) => card_defaults::private(&self.rules.beneath(&venue)),
            None => Value::Null,
        }
    }

    /// `cards.save`：查过、写进盘、换上。
    fn cards_save(&mut self, id: &Value, args: &Value) {
        let answerer = self.core.answerer();
        let Ok(cards) = serde_json::from_value::<Cards>(args["cards"].clone()) else {
            tracing::info!(target: TARGET, "venue cards refused, bad params");
            answerer.refuse(id, "bad_params");
            return;
        };
        let problems: Vec<Value> = cards
            .problems()
            .iter()
            .map(|problem| {
                json!({
                    "card": Card::parse(&problem.file).map(Card::json),
                    "key": problem.key,
                    "code": problem.code.as_str(),
                    "got": problem.got,
                })
            })
            .collect();
        if !problems.is_empty() {
            tracing::info!(target: TARGET, problems = problems.len(), "venue cards refused");
            answerer.answer(id, json!({"saved": false, "problems": problems}));
            return;
        }
        if let Err(error) = save_cards(self.rules.root(), &cards) {
            tracing::warn!(target: TARGET, error = %error, "venue cards not saved");
            answerer.refuse(id, "not_saved");
            return;
        }
        tracing::info!(
            target: TARGET,
            groups = cards.groups.len(),
            privates = cards.privates.len(),
            "venue cards saved"
        );
        self.rules.set_cards(cards);
        answerer.answer(id, json!({"saved": true}));
        // 在看两页的照新的推（施工 O-58 下，`pages`）。
        self.repush_pages();
    }
}

/// 问齐她在的群、好友、人格的名字，和手头的 `known` 一起答。
async fn listed(id: Value, known: Value, bots: Arc<Bots>, caller: Caller, answerer: Answerer) {
    let lists = lists(&bots, &caller).await;
    let mut answer = known;
    answer["groups"] = Value::Array(lists.groups);
    answer["friends"] = Value::Array(lists.friends);
    answer["personas"] = Value::Array(lists.personas);
    answerer.answer(&id, answer);
}

/// 她在的群（`{id, name, members}`）、好友（`{id, name}`）、人格（`{persona, name}`）。
pub(super) struct Lists {
    pub(super) groups: Vec<Value>,
    pub(super) friends: Vec<Value>,
    pub(super) personas: Vec<Value>,
}

/// 问齐她在的群、好友（连着的号都问 NapCat）、人格的名字（核心的 `persona.list`，写错的不列）：问不到的是空的。
pub(super) async fn lists(bots: &Bots, caller: &Caller) -> Lists {
    let mut groups = Vec::new();
    let mut friends = Vec::new();
    for (_, link) in bots.all() {
        if let Ok(reply) = link
            .calls
            .call(&link.out, "get_group_list", json!({}))
            .await
        {
            groups.extend(entries(
                &reply["data"],
                "group_id",
                "group_name",
                Some("member_count"),
            ));
        }
        if let Ok(reply) = link
            .calls
            .call(&link.out, "get_friend_list", json!({}))
            .await
        {
            friends.extend(entries(&reply["data"], "user_id", "nickname", None));
        }
    }
    let personas = match caller.call("persona.list", json!({})).await {
        Ok(reply) if reason(&reply).is_none() => reply["result"]["personas"]
            .as_array()
            .map(|list| {
                list.iter()
                    .filter(|persona| persona.get("problem").is_none())
                    .map(|persona| json!({"persona": persona["persona"], "name": persona["name"]}))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    Lists {
        groups,
        friends,
        personas,
    }
}

/// NapCat 回的一张表里每一行的号、名字（和人数）：号不是整数的不要；同一个号只留第一行（几个号都在一个群里）。
fn entries(data: &Value, id: &str, name: &str, count: Option<&str>) -> Vec<Value> {
    let mut seen = std::collections::BTreeSet::new();
    data.as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let number = row[id].as_u64()?;
                    seen.insert(number).then(|| {
                        let mut entry = json!({"id": number, "name": row[name]});
                        if let Some(count) = count {
                            entry["members"] = row[count].clone();
                        }
                        entry
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
