//! 后台页的场所卡片（施工 O-58 上，`onebot.md` 第一条「场所卡片」）：`cards.get`、`cards.save` 两个方法，读的一头照推送交到这里，
//! 卡片、场所规则、连接都在这一头。
//!
//! - `cards.get`：卡片、每一格的默认值（卡片下面几层套出来的，`rules::card_defaults`；私聊的有卡的、白名单成员都给）、白名单成员的号当场交；她在的群、好友
//!   （NapCat 的 `get_group_list`、`get_friend_list`，连着的号都问）和人格的名字（核心的 `persona.list`）另起任务问，问不到的
//!   照空的交，页面写号。
//! - `cards.save {cards}`：读不成卡片的样子拒 `bad_params`；拼成规则有问题的不存，答 `{saved: false, problems}`，问题一条条
//!   交回（哪一张卡、哪一格、原因码、原文；不拒：核心转方法的错误只带原因，带不过 `data`）；写不进盘的拒 `not_saved`；存好了
//!   答 `{saved: true}`、当场换上，下一条消息就照新的套。

use std::sync::Arc;

use serde_json::{Map, Value, json};

use super::Route;
use crate::TARGET;
use crate::chat::Bots;
use crate::core::Answerer;
use crate::core::caller::Caller;
use crate::core::methods::{CARDS_GET, CARDS_SAVE};
use crate::core::reason;
use crate::plugins::Toggles;
use crate::rules::Applied;
use crate::rules::{Card, Cards, card_defaults, save_cards};
use miyu_chat::{Venue, VenueKind};

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
        // 私聊的：有卡的和白名单成员（施工 I-7 上：会话页上白名单成员还没有卡的也是一张）。
        let whitelist = self.whitelist_numbers();
        let mut privates = Map::new();
        for id in cards
            .privates
            .iter()
            .map(|card| card.id)
            .chain(whitelist.iter().copied())
        {
            privates.insert(id.to_string(), self.private_defaults(id));
        }
        let defaults = json!({
            // 「其他群」：照一个不在任何名单里的群号套（群号 0 不是真群）。
            "other_groups": self.group_defaults(0),
            "other_privates": self.other_private_defaults(),
            // 会话默认值那张卡自己的默认值（施工 I-7 下二）：卡片都不算，照一个群算。
            "defaults": self.defaults_defaults(),
            "groups": groups,
            "privates": privates,
        });
        // 认得的 im 插件（施工 I-7 中四：会话浮窗「管理插件」）。
        let plugins = self.hooks.listed();
        let known = json!({"cards": cards, "defaults": defaults, "whitelist": whitelist, "plugins": plugins});
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
            .and_then(|number| self.platform.venue(VenueKind::Group, number).ok())
        {
            Some(venue) => self.with_rules(card_defaults::group, &venue),
            None => Value::Null,
        }
    }

    /// 「其他私聊」卡每一格的默认值（施工 O-60）：照一个不是谁的私聊（号 0）套。
    pub(super) fn other_private_defaults(&self) -> Value {
        match self.platform.venue(VenueKind::Private, 0) {
            Ok(venue) => self.with_rules(card_defaults::other_private, &venue),
            Err(_) => Value::Null,
        }
    }

    /// 对方的号是 `user` 的私聊卡每一格的默认值。
    pub(super) fn private_defaults(&self, user: u64) -> Value {
        match i64::try_from(user)
            .ok()
            .and_then(|number| self.platform.venue(VenueKind::Private, number).ok())
        {
            Some(venue) => self.with_rules(card_defaults::private, &venue),
            None => Value::Null,
        }
    }

    /// 会话默认值那张卡每一格的默认值（施工 I-7 下二）：卡片都不算（`beneath_defaults`），照群号 0 的群套（不是真群）。
    fn defaults_defaults(&self) -> Value {
        match self.platform.venue(VenueKind::Group, 0) {
            Ok(venue) => {
                let beneath = self.rules.beneath_defaults(&venue);
                let mut found = card_defaults::group(&beneath);
                found["rules"] = card_defaults::rules(&beneath, self.rules.factory_values());
                found
            }
            Err(_) => Value::Null,
        }
    }

    /// 场所 `venue` 上一种卡的默认值（`defaults`）带上卡上 `rules` 能写的每一个键的（施工 I-7 中一，`card_defaults::rules`）。
    fn with_rules(&self, defaults: fn(&Applied) -> Value, venue: &Venue) -> Value {
        let beneath = self.rules.beneath(venue);
        let mut found = defaults(&beneath);
        found["rules"] = card_defaults::rules(&beneath, self.rules.factory_values());
        found
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
        let before = self.wanted();
        self.rules.set_cards(cards);
        self.repin(&before);
        self.hooks
            .set_toggles(Toggles::from_cards(self.rules.cards()));
        self.forget_privates();
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

/// 她在的群（`{id, name, members, bot}`）、好友（`{id, name, bot}`，`bot` 是问到它的那个号，施工 I-7 上：会话卡上写是哪个
/// 机器人）、人格（`{persona, name}`）。
pub(super) struct Lists {
    pub(super) groups: Vec<Value>,
    pub(super) friends: Vec<Value>,
    pub(super) personas: Vec<Value>,
}

/// 问齐她在的群、好友（连着的号都问 NapCat）、人格的名字（核心的 `persona.list`，写错的不列）：问不到的是空的。
pub(super) async fn lists(bots: &Bots, caller: &Caller) -> Lists {
    let mut groups = Vec::new();
    let mut friends = Vec::new();
    for (bot, link) in bots.all() {
        if let Ok(list) = link.groups().await {
            let rows = list.into_iter().map(|group| {
                let row = json!({"id": group.id, "name": group.name, "members": group.members, "bot": bot});
                (group.id, row)
            });
            groups.extend(entries(rows));
        }
        if let Ok(list) = link.friends().await {
            let rows = list.into_iter().map(|friend| {
                (
                    friend.id,
                    json!({"id": friend.id, "name": friend.name, "bot": bot}),
                )
            });
            friends.extend(entries(rows));
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

/// 一张表里每一行（号、交出去的样子）：同一个号只留第一行。
fn entries(rows: impl Iterator<Item = (i64, Value)>) -> Vec<Value> {
    let mut seen = std::collections::BTreeSet::new();
    rows.filter(|(id, _)| seen.insert(*id))
        .map(|(_, entry)| entry)
        .collect()
}

#[cfg(test)]
mod tests;
