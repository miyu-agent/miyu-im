//! 卡片照作用范围（平台、一个号）找、改（施工 I-81 中，设计 07 第 7.6 节；从 `cards.rs` 分出来）：「其他群」「其他私聊」照范围存在
//! `others` 里，没写的号照整个平台的；一个连接看到的那一份；有自己卡片的号。

use std::collections::{BTreeMap, BTreeSet};

use super::{
    Cards, GroupCard, GroupSet, OLD_PLATFORM, OtherPrivateSet, Others, PrivateCard, Scope,
};
use crate::chat::Id;

/// `others` 里的键（施工 I-81 中）：整个平台写平台名，只管一个号的写 `<平台>:<机器人号>`。
pub fn others_key(scope: &Scope) -> String {
    match &scope.bot {
        Some(bot) => format!("{}:{bot}", scope.platform),
        None => scope.platform.clone(),
    }
}

/// [`others_key`] 的反面：机器人号写错的是空的。平台名合不合写法不在这里查（[`Cards::bad_platforms`]）。
pub(super) fn scope_of_key(key: &str) -> Option<Scope> {
    match key.split_once(':') {
        Some((platform, bot)) => Some(Scope::of(platform, Some(Id::new(bot)?))),
        None => Some(Scope::platform(key)),
    }
}

impl Cards {
    /// `scope` 写下的「其他群」「其他私聊」：整个 [`OLD_PLATFORM`] 的在顶上两格，别的在 `others` 里；没写的是空的。
    fn others_written(&self, scope: &Scope) -> Option<(&GroupSet, &OtherPrivateSet)> {
        if scope.bot.is_none() && scope.platform == OLD_PLATFORM {
            return Some((&self.other_groups, &self.other_privates));
        }
        self.others
            .get(&others_key(scope))
            .map(|others| (&others.groups, &others.privates))
    }

    /// `scope` 的「其他群」：写了的照写的；只管一个号的没写照整个平台的（设计 07 第 7.6 节）；都没写的是空卡。
    pub fn other_groups_of(&self, scope: &Scope) -> GroupSet {
        self.others_written(scope)
            .or_else(|| self.others_written(&Scope::platform(&scope.platform)))
            .map(|(groups, _)| groups.clone())
            .unwrap_or_default()
    }

    /// `scope` 的「其他私聊」：同 [`Cards::other_groups_of`]。
    pub fn other_privates_of(&self, scope: &Scope) -> OtherPrivateSet {
        self.others_written(scope)
            .or_else(|| self.others_written(&Scope::platform(&scope.platform)))
            .map(|(_, privates)| privates.clone())
            .unwrap_or_default()
    }

    /// 改 `scope` 的「其他群」「其他私聊」那一份：只管一个号的、还没写过的先照整个平台的抄一份（改了只管这个号，项目主人 10-30 定）。
    fn others_mut(&mut self, scope: &Scope) -> (&mut GroupSet, &mut OtherPrivateSet) {
        if scope.bot.is_none() && scope.platform == OLD_PLATFORM {
            return (&mut self.other_groups, &mut self.other_privates);
        }
        let key = others_key(scope);
        if !self.others.contains_key(&key) {
            let whole = Scope::platform(&scope.platform);
            let copied = Others {
                groups: self.other_groups_of(&whole),
                privates: self.other_privates_of(&whole),
            };
            self.others.insert(key.clone(), copied);
        }
        let others = self.others.entry(key).or_default();
        (&mut others.groups, &mut others.privates)
    }

    /// 改 `scope` 的「其他群」那一份：只管一个号的还没有自己那一张的，先照整个平台的抄一张。
    pub fn other_groups_mut(&mut self, scope: &Scope) -> &mut GroupSet {
        self.others_mut(scope).0
    }

    /// 改 `scope` 的「其他私聊」那一份（同 [`Cards::other_groups_mut`]）。
    pub fn other_privates_mut(&mut self, scope: &Scope) -> &mut OtherPrivateSet {
        self.others_mut(scope).1
    }

    /// 一个连接看到的那一份（施工 I-85 中、I-81 中，会话页照连接看）：`scope` 的平台上，管整个平台的和只管这个号的群卡、私聊卡（同一个
    /// 号两张都有的只放这个号的），「其他群」「其他私聊」两格放这个号的（没写的照整个平台的）；会话默认值不分平台。是给人看的一份，
    /// 不拿来整份存（存了别的平台、别的号的就丢了）。`scope` 没写号的只看整个平台的卡。
    pub fn of_scope(&self, scope: &Scope) -> Cards {
        let visible = |platform: &str, bot: &Option<Id>| {
            platform == scope.platform && (bot.is_none() || *bot == scope.bot)
        };
        let own = |bot: &Option<Id>| bot.is_some() && *bot == scope.bot;
        let mut groups: Vec<GroupCard> = Vec::new();
        for card in self
            .groups
            .iter()
            .filter(|card| visible(&card.platform, &card.bot))
        {
            let mine = own(&card.bot);
            match groups.iter().position(|one| one.id == card.id) {
                Some(at) if mine => groups[at] = card.clone(),
                Some(_) => {}
                None => groups.push(card.clone()),
            }
        }
        let mut privates: Vec<PrivateCard> = Vec::new();
        for card in self
            .privates
            .iter()
            .filter(|card| visible(&card.platform, &card.bot))
        {
            let mine = own(&card.bot);
            match privates.iter().position(|one| one.id == card.id) {
                Some(at) if mine => privates[at] = card.clone(),
                Some(_) => {}
                None => privates.push(card.clone()),
            }
        }
        Cards {
            other_groups: self.other_groups_of(scope),
            groups,
            privates,
            other_privates: self.other_privates_of(scope),
            others: BTreeMap::new(),
            defaults: self.defaults.clone(),
        }
    }

    /// 平台 `platform` 上有自己卡片的号（只管它的群卡、私聊卡，或者 `others` 里有它的一份），照号排、不重。
    pub(super) fn bots_on(&self, platform: &str) -> BTreeSet<Id> {
        let carded = self
            .groups
            .iter()
            .filter(|card| card.platform == platform)
            .filter_map(|card| card.bot.clone())
            .chain(
                self.privates
                    .iter()
                    .filter(|card| card.platform == platform)
                    .filter_map(|card| card.bot.clone()),
            );
        let others = self
            .others
            .keys()
            .filter_map(|key| scope_of_key(key))
            .filter(|scope| scope.platform == platform)
            .filter_map(|scope| scope.bot);
        carded.chain(others).collect()
    }

    /// 「陌生群」「陌生私聊」各有一张的范围（插件开关照它查，`plugins::Toggles`）：卡片上提到的平台的整个平台，加上有自己卡片的号。
    pub fn stranger_scopes(&self) -> Vec<Scope> {
        let mut scopes = Vec::new();
        for platform in self.platforms(&[]) {
            scopes.push(Scope::platform(platform));
            for bot in self.bots_on(platform) {
                scopes.push(Scope::of(platform, Some(bot)));
            }
        }
        scopes
    }
}
