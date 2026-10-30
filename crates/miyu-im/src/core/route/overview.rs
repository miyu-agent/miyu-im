//! 会话浮窗的「概况」（施工 I-7 上补，设计 01 第八节第 7 条：是否回复、今日消息、最近判断）：`cards.overview {card}`，卡的写法同
//! `cards.save_one`（`card_edit::card_of`）。答 `{today: {received, replied, recorded, undelivered}, recent: [{at, who, text, outcome,
//! why}]}`：今天的数照 `crate::counters` 这张卡的场所加起来，最近判断照 `crate::recent`，新的在前。
//!
//! - 群卡、私聊卡：那一个场所。
//! - 「陌生群」：名单以外的群并起来；「陌生私聊」：不在私聊白名单里的人的私聊并起来（终端管理员的私聊也算在里面）。
//! - 卡写错的拒 `bad_params`。

use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::Route;
use crate::core::methods::CARDS_OVERVIEW;
use crate::rules::{Card, Scope, card_edit};
use miyu_chat::{Venue, VenueKind};
use miyu_client::ids::VenueId;

/// 认一个场所编号是不是这张卡管的。
type Wanted = Box<dyn Fn(&str) -> bool>;

impl Route {
    /// 核心转来的一次 `method.call`（`pushed`）是不是 `cards.overview`：是的照答，交回真。
    pub(super) fn overview_called(&mut self, pushed: &Value) -> bool {
        if pushed["params"]["method"] != CARDS_OVERVIEW {
            return false;
        }
        let answerer = self.core.answerer();
        match self.overview(&pushed["params"]["params"]) {
            Some(answer) => answerer.answer(&pushed["id"], answer),
            None => answerer.refuse(&pushed["id"], "bad_params"),
        }
        true
    }

    /// 一张卡的概况；卡写错的是空的。平台照 `args`（施工 I-80，同 `cards.save_one`）。
    fn overview(&self, args: &Value) -> Option<Value> {
        let platform = self
            .platforms
            .ours(args)
            .ok()
            .map(|platform| platform.name());
        let card = card_edit::card_of(&args["card"], platform)?;
        let wanted = self.wanted_venues(card)?;
        let today = self.counters.today_of(&wanted);
        let recent: Vec<Value> = self
            .recent
            .of(&wanted)
            .into_iter()
            .map(|(_, note)| json!(note))
            .collect();
        Some(json!({"today": today, "recent": recent}))
    }

    /// 一张卡管的场所：交回认场所编号的那一段。场所编号解开了照（平台、机器人号、种类、号）比（施工 I-81：只管一个号的卡只认那个号
    /// 的场所，管整个平台的认这个平台所有的号）。
    fn wanted_venues(&self, card: Card) -> Option<Wanted> {
        Some(match card {
            // 会话默认值不是哪个场所：没有概况。
            Card::Defaults => return None,
            Card::Group(scope, id) => {
                let id = id.to_string();
                Box::new(move |venue: &str| {
                    parts(venue).is_some_and(|seen| {
                        seen.within(&scope) && seen.kind == VenueKind::Group && seen.number == id
                    })
                })
            }
            Card::Private(scope, id) => {
                let id = id.to_string();
                Box::new(move |venue: &str| {
                    parts(venue).is_some_and(|seen| {
                        seen.within(&scope) && seen.kind == VenueKind::Private && seen.number == id
                    })
                })
            }
            Card::OtherGroups(scope) => {
                // 名单：整个平台的群卡，加上这个号自己的（同拼规则，设计 07 第 7.6 节）。
                let listed: BTreeSet<String> = self
                    .rules
                    .cards()
                    .groups
                    .iter()
                    .filter(|card| card.platform == scope.platform)
                    .filter(|card| card.bot.is_none() || card.bot == scope.bot)
                    .map(|card| card.id.to_string())
                    .collect();
                Box::new(move |venue: &str| {
                    parts(venue).is_some_and(|seen| {
                        seen.within(&scope)
                            && seen.kind == VenueKind::Group
                            && !listed.contains(&seen.number)
                    })
                })
            }
            Card::OtherPrivates(scope) => {
                let listed: BTreeSet<String> = self
                    .whitelist_numbers(&scope.platform)
                    .into_iter()
                    .map(|id| id.to_string())
                    .collect();
                Box::new(move |venue: &str| {
                    parts(venue).is_some_and(|seen| {
                        seen.within(&scope)
                            && seen.kind == VenueKind::Private
                            && !listed.contains(&seen.number)
                    })
                })
            }
        })
    }
}

/// 解开的场所编号。
struct Seen {
    platform: String,
    bot: Option<String>,
    kind: VenueKind,
    number: String,
}

impl Seen {
    /// 在不在卡的范围 `scope` 里：平台对得上，只管一个号的还得是那个号。
    fn within(&self, scope: &Scope) -> bool {
        self.platform == scope.platform
            && scope
                .bot
                .as_ref()
                .is_none_or(|bot| self.bot.as_deref() == Some(bot.as_str()))
    }
}

/// 场所编号 `venue` 解开；解不开的是空的。
fn parts(venue: &str) -> Option<Seen> {
    let venue = Venue::parse(&VenueId::parse(venue).ok()?)?;
    Some(Seen {
        platform: venue.platform().to_string(),
        bot: venue.bot().map(str::to_string),
        kind: venue.kind(),
        number: venue.number().to_string(),
    })
}
