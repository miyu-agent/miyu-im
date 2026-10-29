//! 场所开关（施工 I-8 上，设计 01 第四节第 4 条）：一个场所开没开一个插件，照场所卡片的 `plugins` 那一格：这个场所的卡上写了的
//! 照它；没写的照兜底卡（群照「陌生群」，私聊照「陌生私聊」）；兜底卡也没写的照插件声明的 `default`。卡片读进来、存下时
//! 照它换一份（[`Toggles::from_cards`]），挂接点照它查。

use std::collections::BTreeMap;

use miyu_chat::VenueKind;

use crate::chat::Id;
use crate::rules::Cards;

/// 包的编号到开没开。
type Switches = BTreeMap<String, bool>;

/// 照卡片拼好的、按场所查的开关。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Toggles {
    /// 群号到它卡上的开关。
    groups: BTreeMap<Id, Switches>,
    /// 私聊对方的号到它卡上的开关。
    privates: BTreeMap<Id, Switches>,
    /// 「陌生群」卡上的。
    other_groups: Switches,
    /// 「陌生私聊」卡上的。
    other_privates: Switches,
}

impl Toggles {
    /// 照 `cards` 拼。
    pub fn from_cards(cards: &Cards) -> Toggles {
        let switches = |set: &Option<Switches>| set.clone().unwrap_or_default();
        Toggles {
            groups: cards
                .groups
                .iter()
                .map(|card| (card.id.clone(), switches(&card.set.plugins)))
                .collect(),
            privates: cards
                .privates
                .iter()
                .map(|card| (card.id.clone(), switches(&card.set.plugins)))
                .collect(),
            other_groups: switches(&cards.other_groups.plugins),
            other_privates: switches(&cards.other_privates.plugins),
        }
    }

    /// 种类是 `kind`、号是 `number` 的场所开没开包 `package`（声明的默认是 `default`）。场所认不出的（`None`）照 `default`。
    pub fn enabled(&self, venue: Option<(VenueKind, &Id)>, package: &str, default: bool) -> bool {
        let Some((kind, number)) = venue else {
            return default;
        };
        let (cards, fallback) = match kind {
            VenueKind::Group => (&self.groups, &self.other_groups),
            VenueKind::Private => (&self.privates, &self.other_privates),
        };
        cards
            .get(number)
            .and_then(|switches| switches.get(package))
            .or_else(|| fallback.get(package))
            .copied()
            .unwrap_or(default)
    }
}
