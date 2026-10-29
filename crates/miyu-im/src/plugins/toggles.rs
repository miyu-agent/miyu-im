//! 场所开关（施工 I-8 上，设计 01 第四节第 4 条）：一个场所开没开一个插件，照场所卡片的 `plugins` 那一格：这个场所的卡上写了的
//! 照它；没写的照兜底卡（群照「陌生群」，私聊照「陌生私聊」）；兜底卡也没写的照插件声明的 `default`。卡片读进来、存下时
//! 照它换一份（[`Toggles::from_cards`]），挂接点照它查。

use std::collections::BTreeMap;

use miyu_chat::VenueKind;

use crate::chat::Id;
use crate::rules::{Cards, OLD_PLATFORM};

/// 包的编号到开没开。
type Switches = BTreeMap<String, bool>;

/// 照卡片拼好的、按场所查的开关。键带平台（施工 I-80）：几个平台的号一样也不串。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Toggles {
    /// （平台，群号）到它卡上的开关。
    groups: BTreeMap<(String, Id), Switches>,
    /// （平台，私聊对方的号）到它卡上的开关。
    privates: BTreeMap<(String, Id), Switches>,
    /// 平台到它的「陌生群」卡上的。
    other_groups: BTreeMap<String, Switches>,
    /// 平台到它的「陌生私聊」卡上的。
    other_privates: BTreeMap<String, Switches>,
}

impl Toggles {
    /// 照 `cards` 拼。
    pub fn from_cards(cards: &Cards) -> Toggles {
        let switches = |set: &Option<Switches>| set.clone().unwrap_or_default();
        let mut toggles = Toggles {
            groups: cards
                .groups
                .iter()
                .map(|card| {
                    (
                        (card.platform.clone(), card.id.clone()),
                        switches(&card.set.plugins),
                    )
                })
                .collect(),
            privates: cards
                .privates
                .iter()
                .map(|card| {
                    (
                        (card.platform.clone(), card.id.clone()),
                        switches(&card.set.plugins),
                    )
                })
                .collect(),
            ..Toggles::default()
        };
        let platforms = cards
            .others
            .keys()
            .map(String::as_str)
            .chain([OLD_PLATFORM]);
        for platform in platforms {
            let groups = switches(&cards.other_groups_of(platform).plugins);
            let privates = switches(&cards.other_privates_of(platform).plugins);
            toggles.other_groups.insert(platform.to_string(), groups);
            toggles
                .other_privates
                .insert(platform.to_string(), privates);
        }
        toggles
    }

    /// 平台 `platform` 上种类是 `kind`、号是 `number` 的场所开没开包 `package`（声明的默认是 `default`）。场所认不出的
    /// （`None`）照 `default`。
    pub fn enabled(
        &self,
        venue: Option<(&str, VenueKind, &Id)>,
        package: &str,
        default: bool,
    ) -> bool {
        let Some((platform, kind, number)) = venue else {
            return default;
        };
        let (cards, fallback) = match kind {
            VenueKind::Group => (&self.groups, &self.other_groups),
            VenueKind::Private => (&self.privates, &self.other_privates),
        };
        cards
            .get(&(platform.to_string(), number.clone()))
            .and_then(|switches| switches.get(package))
            .or_else(|| fallback.get(platform)?.get(package))
            .copied()
            .unwrap_or(default)
    }
}
