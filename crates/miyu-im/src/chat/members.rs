//! 群里的身份和群成员名字、身份的缓存（施工 I-2 上从 `im/members.rs` 挪来）：按平台、按群、按号记，记多久由用的一方定。键带平台
//! （施工 I-70 上）：QQ 群和 Telegram 群号码一样时不串名字。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::{Id, Platform};

/// 一个人在群里的身份（施工 O-31）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rank {
    /// 群主：`owner`。
    Owner,
    /// 群管理员：`admin`。
    Admin,
    /// 普通成员：`member`。
    Member,
}

impl Rank {
    /// 给模型看的写法：`owner`、`admin`、`member`（群成员列表照它写，施工 O-54）。
    pub fn word(self) -> &'static str {
        match self {
            Rank::Owner => "owner",
            Rank::Admin => "admin",
            Rank::Member => "member",
        }
    }
}

/// 群成员的名字缓存。
#[derive(Debug)]
pub struct Members {
    /// 记多久。
    keep: Duration,
    /// （平台，群号，号）→（名字，记下的时刻）。
    names: HashMap<(Platform, Id, Id), (String, Instant)>,
    /// （平台，群号，号）→（身份，记下的时刻）（施工 O-31）。
    ranks: HashMap<(Platform, Id, Id), (Rank, Instant)>,
    /// （平台，机器人号）→ 它的 QQ 昵称、记下的时刻（施工 I-38：私聊里她叫什么）。
    nicknames: HashMap<(Platform, Id), (String, Instant)>,
    /// （平台，机器人号，群号；私聊的没有群号）→ 上次去问她叫什么的时刻（施工 I-38）：问不到的，`keep` 以内不再问。
    asked: HashMap<(Platform, Id, Option<Id>), Instant>,
}

impl Members {
    /// 一个空的缓存，名字记 `keep`。
    pub fn new(keep: Duration) -> Members {
        Members {
            keep,
            names: HashMap::new(),
            ranks: HashMap::new(),
            nicknames: HashMap::new(),
            asked: HashMap::new(),
        }
    }

    /// 平台 `platform` 上群 `group` 此刻 `now` 记着的每一个人的名字（号 → 名字，施工 T-2：交给驱动的工具写结果里的人）。
    pub fn names_in(&self, platform: Platform, group: &Id, now: Instant) -> HashMap<Id, String> {
        self.names
            .iter()
            .filter(|((on, one, _), (_, at))| {
                *on == platform && one == group && now.saturating_duration_since(*at) < self.keep
            })
            .map(|((_, _, user), (name, _))| (user.clone(), name.clone()))
            .collect()
    }

    /// 平台 `platform` 上群 `group` 里号是 `user` 的人此刻 `now` 叫 `name`：记下（盖掉原来的），顺手扔掉过了的，缓存不会越攒越多。
    pub fn remember(
        &mut self,
        (platform, group, user): (Platform, Id, Id),
        name: String,
        now: Instant,
    ) {
        let keep = self.keep;
        self.names
            .retain(|_, (_, at)| now.saturating_duration_since(*at) < keep);
        self.names.insert((platform, group, user), (name, now));
    }

    /// 平台 `platform` 上群 `group` 里号是 `user` 的人叫什么：记下以后到此刻 `now` 还不到 `keep` 的才有。
    pub fn name(&self, (platform, group, user): (Platform, Id, Id), now: Instant) -> Option<&str> {
        self.names
            .get(&(platform, group, user))
            .filter(|(_, at)| now.saturating_duration_since(*at) < self.keep)
            .map(|(name, _)| name.as_str())
    }

    /// 平台 `platform` 上群 `group` 里号是 `user` 的人记着的名字不要了（施工 I-38：群名片清掉了，下回问到的是昵称）。是她自己的，
    /// 「这个群刚问过」也一起忘掉（[`Members::ask_due`]），下一条就去问，不等缓存时长过去。
    pub fn forget(&mut self, (platform, group, user): (Platform, Id, Id)) {
        self.asked
            .remove(&(platform, user.clone(), Some(group.clone())));
        self.names.remove(&(platform, group, user));
    }

    /// 平台 `platform` 上机器人号 `bot` 此刻 `now` 的昵称是 `name`：记下（施工 I-38）。
    pub fn nicknamed(&mut self, (platform, bot): (Platform, Id), name: String, now: Instant) {
        self.nicknames.insert((platform, bot), (name, now));
    }

    /// 平台 `platform` 上机器人号 `bot` 的昵称：同 [`Members::name`]，过了的不用。
    pub fn nickname(&self, (platform, bot): (Platform, Id), now: Instant) -> Option<&str> {
        self.nicknames
            .get(&(platform, bot))
            .filter(|(_, at)| now.saturating_duration_since(*at) < self.keep)
            .map(|(name, _)| name.as_str())
    }

    /// 此刻 `now` 该不该去问平台 `platform` 上机器人号 `bot` 在群 `group`（私聊的没有）里叫什么：`keep` 以内问过的不该；该的记下
    /// 这一次。
    pub fn ask_due(
        &mut self,
        (platform, bot): (Platform, Id),
        group: Option<Id>,
        now: Instant,
    ) -> bool {
        let keep = self.keep;
        self.asked
            .retain(|_, at| now.saturating_duration_since(*at) < keep);
        match self.asked.entry((platform, bot, group)) {
            std::collections::hash_map::Entry::Occupied(_) => false,
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(now);
                true
            }
        }
    }

    /// 平台 `platform` 上群 `group` 里号是 `user` 的人此刻 `now` 的身份是 `rank`（施工 O-31）：记下（盖掉原来的），顺手扔掉过了的。
    pub fn ranked(
        &mut self,
        (platform, group, user): (Platform, Id, Id),
        rank: Rank,
        now: Instant,
    ) {
        let keep = self.keep;
        self.ranks
            .retain(|_, (_, at)| now.saturating_duration_since(*at) < keep);
        self.ranks.insert((platform, group, user), (rank, now));
    }

    /// 平台 `platform` 上群 `group` 里号是 `user` 的人的身份：同 [`Members::name`]，过了的不用。
    pub fn rank(&self, (platform, group, user): (Platform, Id, Id), now: Instant) -> Option<Rank> {
        self.ranks
            .get(&(platform, group, user))
            .filter(|(_, at)| now.saturating_duration_since(*at) < self.keep)
            .map(|(rank, _)| *rank)
    }
}
