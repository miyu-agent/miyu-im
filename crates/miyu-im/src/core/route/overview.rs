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
use crate::chat::Id;
use crate::core::methods::CARDS_OVERVIEW;
use crate::rules::{Card, card_edit};
use miyu_chat::VenueKind;

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

    /// 一张卡管的场所：交回认场所编号的那一段。
    fn wanted_venues(&self, card: Card) -> Option<Wanted> {
        let key = |platform: &str, kind, number: &Id| -> Option<String> {
            Some(Self::card_venue(platform, kind, number)?.id().to_string())
        };
        // 一种场所的编号的开头：照号 0 的编号去掉最后一段（`qq:group:`）。
        let prefix = |platform: &str, kind| -> Option<String> {
            let one = key(platform, kind, &Id::from(0))?;
            Some(format!("{}:", one.rsplit_once(':')?.0))
        };
        Some(match card {
            // 会话默认值不是哪个场所：没有概况。
            Card::Defaults => return None,
            Card::Group(platform, id) => {
                let one = key(&platform, VenueKind::Group, &id)?;
                Box::new(move |venue: &str| venue == one)
            }
            Card::Private(platform, id) => {
                let one = key(&platform, VenueKind::Private, &id)?;
                Box::new(move |venue: &str| venue == one)
            }
            Card::OtherGroups(platform) => {
                let listed: BTreeSet<String> = self
                    .rules
                    .cards()
                    .groups
                    .iter()
                    .filter(|card| card.platform == platform)
                    .filter_map(|card| key(&platform, VenueKind::Group, &card.id))
                    .collect();
                let start = prefix(&platform, VenueKind::Group)?;
                Box::new(move |venue: &str| venue.starts_with(&start) && !listed.contains(venue))
            }
            Card::OtherPrivates(platform) => {
                let listed: BTreeSet<String> = self
                    .whitelist_numbers(&platform)
                    .into_iter()
                    .filter_map(|id| key(&platform, VenueKind::Private, &id))
                    .collect();
                let start = prefix(&platform, VenueKind::Private)?;
                Box::new(move |venue: &str| venue.starts_with(&start) && !listed.contains(venue))
            }
        })
    }
}
