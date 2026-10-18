//! 群里的身份和群成员名字、身份的缓存（施工 I-2 上从 `onebot/members.rs` 挪来）：按群、按号记，记多久由用的一方定。

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
}

impl Members {
    /// 一个空的缓存，名字记 `keep`。
    pub fn new(keep: Duration) -> Members {
        Members {
            keep,
            names: HashMap::new(),
            ranks: HashMap::new(),
        }
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
