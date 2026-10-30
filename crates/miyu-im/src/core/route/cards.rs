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
use crate::chat::{Bots, Id};
use crate::core::Answerer;
use crate::core::caller::Caller;
use crate::core::methods::{CARDS_GET, CARDS_SAVE};
use crate::core::reason;
use crate::plugins::Toggles;
use crate::rules::Applied;
use crate::rules::{Card, Cards, OLD_PLATFORM, Scope, card_defaults, save_cards};
use miyu_chat::{Venue, VenueKind};

impl Route {
    /// 核心转来的一次 `method.call`（`pushed`）是不是场所卡片的：是的照方法答，交回真。附加提示词不在这里重放：每一条消息进来
    /// 都照这一刻的规则放（`titled`），回合也只在来了消息以后开。
    pub(super) fn cards_called(&mut self, pushed: &Value) -> bool {
        let id = pushed["id"].clone();
        match pushed["params"]["method"].as_str() {
            Some(CARDS_GET) => self.cards_get(id, &pushed["params"]["params"]),
            Some(CARDS_SAVE) => self.cards_save(&id, &pushed["params"]["params"]),
            _ => return false,
        }
        true
    }

    /// `cards.get {platform?, bot?}`：写了平台的交一个连接看的那一份（施工 I-85 中、I-81 中，会话页照选中的连接问，`Cards::of_scope`）：
    /// 这个平台管整个平台的和只管 `bot` 这个号的卡（同一个号两张都有的只放这个号的），「其他群」「其他私聊」是这个号的（没写的照整个
    /// 平台的），会话默认值不分平台，默认值表的键只写号、照带这个号的场所算；没写平台的照旧交整份（整份读、整份存的 `cards.save`
    /// 不丢别的平台、别的号的卡），默认值表里别的平台的键写成身份。白名单、默认值、群和好友照写的平台（写了号的只问那个号），没写的照
    /// 后台页的平台。手头的当场拼好，群、好友、人格另起任务问齐了再答。
    fn cards_get(&mut self, id: Value, args: &Value) {
        let scoped = args["platform"].as_str();
        let bot = args["bot"].as_str().and_then(Id::new);
        let scope = Scope::of(scoped.unwrap_or(self.page_platform()), bot);
        let platform = scope.platform.as_str();
        let cards = match scoped {
            Some(_) => self.rules.cards().of_scope(&scope),
            None => self.rules.cards().clone(),
        };
        let key = |of: &str, id: &Id| match scoped {
            Some(_) => id.to_string(),
            None => card_key(of, id),
        };
        // 一张卡的默认值照哪儿算：一个连接看的照那个号，整份的照卡自己的。
        let at = |platform: &str, bot: &Option<Id>| match scoped {
            Some(_) => scope.clone(),
            None => Scope::of(platform, bot.clone()),
        };
        let mut groups = Map::new();
        for card in &cards.groups {
            let defaults = self.group_defaults(&at(&card.platform, &card.bot), &card.id);
            groups.insert(key(&card.platform, &card.id), defaults);
        }
        // 私聊的：有卡的和白名单成员（施工 I-7 上：会话页上白名单成员还没有卡的也是一张；I-80 起整份的哪个平台的都算）。
        let mut privates = Map::new();
        let whitelisted = self.whitelist.iter().filter_map(|person| {
            let (of, number) = person.split_once(':')?;
            if scoped.is_some_and(|platform| platform != of) {
                return None;
            }
            Some((Scope::platform(of), Id::new(number)?))
        });
        let carded = cards
            .privates
            .iter()
            .map(|card| (Scope::of(&card.platform, card.bot.clone()), card.id.clone()));
        for (of, id) in carded.chain(whitelisted) {
            let defaults = self.private_defaults(&at(&of.platform, &of.bot), &id);
            privates.insert(key(&of.platform, &id), defaults);
        }
        let whitelist = self.whitelist_numbers(platform);
        let defaults = json!({
            // 「其他群」：照一个不在任何名单里的群号套（群号 0 不是真群）。照问的平台、号（没写的照后台页的平台）。
            "other_groups": other_groups(self.group_defaults(&scope, &Id::from(0))),
            "other_privates": self.other_private_defaults(&scope),
            // 会话默认值那张卡自己的默认值（施工 I-7 下二）：卡片都不算，照一个群算。
            "defaults": self.defaults_defaults(),
            "groups": groups,
            "privates": privates,
        });
        // 认得的 im 插件（施工 I-7 中四：会话浮窗「管理插件」）。
        let plugins = self.hooks.listed();
        let mut known = json!({"platform": platform, "cards": cards, "defaults": defaults, "whitelist": whitelist, "plugins": plugins});
        if let Some(bot) = &scope.bot {
            known["bot"] = json!(bot);
        }
        let bots = self.card_bots(&scope);
        let (caller, answerer) = (self.core.caller(), self.core.answerer());
        self.chores.spawn(listed(id, known, bots, caller, answerer));
    }

    /// 范围 `scope` 上种类是 `kind`、号是 `number` 的场所（施工 I-80：照卡片的平台；I-81 中：只管一个号的照那个号拼，规则里照号写的
    /// 也算得上）。拼不出的是空的。
    pub(super) fn card_venue(scope: &Scope, kind: VenueKind, number: &Id) -> Option<Venue> {
        match &scope.bot {
            Some(bot) => Venue::with_bot(&scope.platform, bot.as_str(), kind, number.as_str()).ok(),
            None => Venue::new(&scope.platform, kind, number.as_str()).ok(),
        }
    }

    /// 后台页照的平台（施工 I-80）：第一个驱动的；一个都没挂的（照说不会）照卡片上没写平台的那一个。页面照连接分是 I-85 的事。
    pub(super) fn page_platform(&self) -> &'static str {
        self.platforms
            .first()
            .map_or(OLD_PLATFORM, |platform| platform.name())
    }

    /// 范围 `scope` 的平台连着的号（施工 I-85 中）；写了号的只要那一个（施工 I-81 中：一个连接看它自己的群、好友）。没挂的是空的。
    pub(super) fn card_bots(&self, scope: &Scope) -> Arc<Bots> {
        let all = self
            .platforms
            .named(&scope.platform)
            .map_or_else(Arc::default, |platform| self.platforms.bots(platform));
        match &scope.bot {
            Some(bot) => Arc::new(all.only(bot)),
            None => all,
        }
    }

    /// 范围 `scope` 上群号是 `group` 的群卡每一格的默认值。
    pub(super) fn group_defaults(&self, scope: &Scope, group: &Id) -> Value {
        match Self::card_venue(scope, VenueKind::Group, group) {
            Some(venue) => self.with_rules(card_defaults::group, &venue),
            None => Value::Null,
        }
    }

    /// 范围 `scope` 的「其他私聊」卡每一格的默认值（施工 O-60）：照一个不是谁的私聊（号 0）套。
    pub(super) fn other_private_defaults(&self, scope: &Scope) -> Value {
        match Self::card_venue(scope, VenueKind::Private, &Id::from(0)) {
            Some(venue) => self.with_rules(card_defaults::other_private, &venue),
            None => Value::Null,
        }
    }

    /// 范围 `scope` 上对方的号是 `user` 的私聊卡每一格的默认值。
    pub(super) fn private_defaults(&self, scope: &Scope, user: &Id) -> Value {
        match Self::card_venue(scope, VenueKind::Private, user) {
            Some(venue) => self.with_rules(card_defaults::private, &venue),
            None => Value::Null,
        }
    }

    /// 会话默认值那张卡每一格的默认值（施工 I-7 下二）：卡片都不算（`beneath_defaults`），照群号 0 的群套（不是真群）。
    fn defaults_defaults(&self) -> Value {
        match Self::card_venue(
            &Scope::platform(self.page_platform()),
            VenueKind::Group,
            &Id::from(0),
        ) {
            Some(venue) => {
                let beneath = self.rules.beneath_defaults(&venue);
                let mut found = card_defaults::group(&beneath);
                found["rules"] = card_defaults::rules(&beneath, self.rules.factory_values());
                found
            }
            None => Value::Null,
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
        // 平台名写错的（施工 I-80）同样子不对：拼不进文件名、规则。
        let read = serde_json::from_value::<Cards>(args["cards"].clone());
        let Ok(cards) = read
            .map_err(drop)
            .and_then(|cards| match cards.bad_platforms()[..] {
                [] => Ok(cards),
                _ => Err(()),
            })
        else {
            tracing::info!(target: TARGET, "venue cards refused, bad params");
            answerer.refuse(id, "bad_params");
            return;
        };
        let problems: Vec<Value> = cards
            .problems()
            .iter()
            .map(|problem| {
                json!({
                    "card": Card::parse(&problem.file).map(|card| card.json()),
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
                (group.id.clone(), row)
            });
            groups.extend(entries(rows));
        }
        if let Ok(list) = link.friends().await {
            let rows = list.into_iter().map(|friend| {
                (
                    friend.id.clone(),
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
fn entries(rows: impl Iterator<Item = (Id, Value)>) -> Vec<Value> {
    let mut seen = std::collections::BTreeSet::new();
    rows.filter(|(id, _)| seen.insert(id.clone()))
        .map(|(_, entry)| entry)
        .collect()
}

/// `cards.get` 交的整份默认值表里一张卡的键（施工 I-80）：QQ 的照旧只写号，别的平台的写成身份 `<平台>:<号>`，几个平台的号一样也不串。
fn card_key(platform: &str, id: &Id) -> String {
    match platform {
        OLD_PLATFORM => id.to_string(),
        _ => format!("{platform}:{id}"),
    }
}

#[cfg(test)]
mod tests;

/// 「其他群」卡的默认值（施工 I-16）：限流只照出厂的那一格时，换成「其他群」那张卡没写时自己补的 [`OTHER_GROUPS_RATE`](crate::rules::OTHER_GROUPS_RATE)（出厂群规则是名单里
/// 的群的 30/60）。
fn other_groups(mut defaults: Value) -> Value {
    if defaults["rate"]["from"] == "factory" {
        defaults["rate"] = json!({"value": crate::rules::OTHER_GROUPS_RATE, "from": "factory"});
    }
    defaults
}
