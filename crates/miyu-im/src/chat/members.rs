//! 群里的身份和群成员名字、身份的缓存（施工 I-2 上从 `im/members.rs` 挪来）：按群、按号记，记多久由用的一方定。

use std::collections::HashMap;
use std::time::{Duration, Instant};

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
    /// （群号，号）→（名字，记下的时刻）。
    names: HashMap<(i64, i64), (String, Instant)>,
    /// （群号，号）→（身份，记下的时刻）（施工 O-31）。
    ranks: HashMap<(i64, i64), (Rank, Instant)>,
    /// 机器人号 → 它的 QQ 昵称、记下的时刻（施工 I-38：私聊里她叫什么）。
    nicknames: HashMap<i64, (String, Instant)>,
    /// （机器人号，群号；私聊的没有群号）→ 上次去问她叫什么的时刻（施工 I-38）：问不到的，`keep` 以内不再问。
    asked: HashMap<(i64, Option<i64>), Instant>,
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

    /// 群 `group` 此刻 `now` 记着的每一个人的名字（号 → 名字，施工 T-2：交给驱动的工具写结果里的人）。
    pub fn names_in(&self, group: i64, now: Instant) -> HashMap<i64, String> {
        self.names
            .iter()
            .filter(|((one, _), (_, at))| {
                *one == group && now.saturating_duration_since(*at) < self.keep
            })
            .map(|((_, user), (name, _))| (*user, name.clone()))
            .collect()
    }

    /// 群 `group` 里号是 `user` 的人此刻 `now` 叫 `name`：记下（盖掉原来的），顺手扔掉过了的，缓存不会越攒越多。
    pub fn remember(&mut self, group: i64, user: i64, name: String, now: Instant) {
        let keep = self.keep;
        self.names
            .retain(|_, (_, at)| now.saturating_duration_since(*at) < keep);
        self.names.insert((group, user), (name, now));
    }

    /// 群 `group` 里号是 `user` 的人叫什么：记下以后到此刻 `now` 还不到 `keep` 的才有。
    pub fn name(&self, group: i64, user: i64, now: Instant) -> Option<&str> {
        self.names
            .get(&(group, user))
            .filter(|(_, at)| now.saturating_duration_since(*at) < self.keep)
            .map(|(name, _)| name.as_str())
    }

    /// 群 `group` 里号是 `user` 的人记着的名字不要了（施工 I-38：群名片清掉了，下回问到的是昵称）。是她自己的，「这个群刚问过」
    /// 也一起忘掉（[`Members::ask_due`]），下一条就去问，不等缓存时长过去。
    pub fn forget(&mut self, group: i64, user: i64) {
        self.names.remove(&(group, user));
        self.asked.remove(&(user, Some(group)));
    }

    /// 机器人号 `bot` 此刻 `now` 的 QQ 昵称是 `name`：记下（施工 I-38）。
    pub fn nicknamed(&mut self, bot: i64, name: String, now: Instant) {
        self.nicknames.insert(bot, (name, now));
    }

    /// 机器人号 `bot` 的 QQ 昵称：同 [`Members::name`]，过了的不用。
    pub fn nickname(&self, bot: i64, now: Instant) -> Option<&str> {
        self.nicknames
            .get(&bot)
            .filter(|(_, at)| now.saturating_duration_since(*at) < self.keep)
            .map(|(name, _)| name.as_str())
    }

    /// 此刻 `now` 该不该去问机器人号 `bot` 在群 `group`（私聊的没有）里叫什么：`keep` 以内问过的不该；该的记下这一次。
    pub fn ask_due(&mut self, bot: i64, group: Option<i64>, now: Instant) -> bool {
        let keep = self.keep;
        self.asked
            .retain(|_, at| now.saturating_duration_since(*at) < keep);
        match self.asked.entry((bot, group)) {
            std::collections::hash_map::Entry::Occupied(_) => false,
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(now);
                true
            }
        }
    }

    /// 群 `group` 里号是 `user` 的人此刻 `now` 的身份是 `rank`（施工 O-31）：记下（盖掉原来的），顺手扔掉过了的。
    pub fn ranked(&mut self, group: i64, user: i64, rank: Rank, now: Instant) {
        let keep = self.keep;
        self.ranks
            .retain(|_, (_, at)| now.saturating_duration_since(*at) < keep);
        self.ranks.insert((group, user), (rank, now));
    }

    /// 群 `group` 里号是 `user` 的人的身份：同 [`Members::name`]，过了的不用。
    pub fn rank(&self, group: i64, user: i64, now: Instant) -> Option<Rank> {
        self.ranks
            .get(&(group, user))
            .filter(|(_, at)| now.saturating_duration_since(*at) < self.keep)
            .map(|(rank, _)| *rank)
    }
}
