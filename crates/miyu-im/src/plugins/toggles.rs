//! 场所开关（施工 I-8 上，设计 01 第四节第 4 条）：一个场所开没开一个插件，照场所卡片的 `plugins` 那一格：这个场所的卡上写了的
//! 照它；没写的照兜底卡（群照「陌生群」，私聊照「陌生私聊」）；兜底卡也没写的照插件声明的 `default`。卡片读进来、存下时
//! 照它换一份（[`Toggles::from_cards`]），挂接点照它查。

use std::collections::BTreeMap;

use miyu_chat::VenueKind;

use crate::chat::Id;
use crate::rules::{Cards, Scope};

/// 包的编号到开没开。
type Switches = BTreeMap<String, bool>;

/// 一张卡管的场所：平台、只管的机器人号（施工 I-81 中，没写的管整个平台）、号。
type Key = (String, Option<Id>, Id);

/// 照卡片拼好的、按场所查的开关。键带平台（施工 I-80）、机器人号（施工 I-81 中）：几个平台、几个号的号一样也不串。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Toggles {
    /// 群卡上的开关。
    groups: BTreeMap<Key, Switches>,
    /// 私聊卡上的开关。
    privates: BTreeMap<Key, Switches>,
    /// 「陌生群」卡上的：整个平台的一张，有自己卡片的号各一张（没写的照整个平台的）。
    other_groups: BTreeMap<Scope, Switches>,
    /// 「陌生私聊」卡上的（同 `other_groups`）。
    other_privates: BTreeMap<Scope, Switches>,
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
                    let key = (card.platform.clone(), card.bot.clone(), card.id.clone());
                    (key, switches(&card.set.plugins))
                })
                .collect(),
            privates: cards
                .privates
                .iter()
                .map(|card| {
                    let key = (card.platform.clone(), card.bot.clone(), card.id.clone());
                    (key, switches(&card.set.plugins))
                })
                .collect(),
            ..Toggles::default()
        };
        for scope in cards.stranger_scopes() {
            let groups = switches(&cards.other_groups_of(&scope).plugins);
            let privates = switches(&cards.other_privates_of(&scope).plugins);
            toggles.other_groups.insert(scope.clone(), groups);
            toggles.other_privates.insert(scope, privates);
        }
        toggles
    }

    /// 平台 `platform` 上机器人号 `bot` 收发的、种类是 `kind`、号是 `number` 的场所开没开包 `package`（声明的默认是 `default`）。
    /// 先照这个号自己的卡，再照整个平台的卡，再照「陌生群」「陌生私聊」（这个号有自己那一张的照它的）。场所认不出的（`None`）照
    /// `default`。
    pub fn enabled(
        &self,
        venue: Option<(&str, Option<&Id>, VenueKind, &Id)>,
        package: &str,
        default: bool,
    ) -> bool {
        let Some((platform, bot, kind, number)) = venue else {
            return default;
        };
        let (cards, fallback) = match kind {
            VenueKind::Group => (&self.groups, &self.other_groups),
            VenueKind::Private => (&self.privates, &self.other_privates),
        };
        let own = |bot: Option<&Id>| {
            cards
                .get(&(platform.to_string(), bot.cloned(), number.clone()))
                .and_then(|switches| switches.get(package))
        };
        let whole = Scope::platform(platform);
        let stranger = bot
            .map(|bot| Scope::of(platform, Some(bot.clone())))
            .filter(|scope| fallback.contains_key(scope))
            .unwrap_or(whole);
        bot.and_then(|bot| own(Some(bot)))
            .or_else(|| own(None))
            .or_else(|| fallback.get(&stranger)?.get(package))
            .copied()
            .unwrap_or(default)
    }
}
