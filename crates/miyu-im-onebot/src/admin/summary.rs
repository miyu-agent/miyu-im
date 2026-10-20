//! 成员汇总和排行榜（施工 Q-4，设计 02 第一节第 4 条）：照管理记录算，纯逻辑；查记录的工具、面板都照它。被禁言几次、总时长、被踢
//! 几次、最近的理由；排行照禁言次数、总时长从多到少，一样的照号。周、月是从此刻往前的 7 天、30 天。

use super::records::{Action, Record};

/// 看多久的。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Period {
    /// 最近 7 天。
    Week,
    /// 最近 30 天。
    Month,
    /// 全部。
    All,
}

impl Period {
    /// 读写法：`week`、`month`、`all`；别的是空的。
    pub(crate) fn read(text: &str) -> Option<Period> {
        match text {
            "week" => Some(Period::Week),
            "month" => Some(Period::Month),
            "all" => Some(Period::All),
            _ => None,
        }
    }

    /// 写法。
    pub(crate) fn name(self) -> &'static str {
        match self {
            Period::Week => "week",
            Period::Month => "month",
            Period::All => "all",
        }
    }

    /// 此刻是 `now`（Unix 秒）时从哪一刻算起。
    fn since(self, now: u64) -> u64 {
        match self {
            Period::Week => now.saturating_sub(7 * 86_400),
            Period::Month => now.saturating_sub(30 * 86_400),
            Period::All => 0,
        }
    }
}

/// 一个人的汇总。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tally {
    /// 号。
    pub user: i64,
    /// 最近一条记录里叫什么；都没有的是空的。
    pub name: Option<String>,
    /// 被禁言几次。
    pub mutes: u32,
    /// 被禁言一共几秒（照禁的时候说的，提前解禁的不减）。
    pub seconds: u64,
    /// 被踢几次。
    pub kicks: u32,
    /// 最近一次禁言、踢人的理由；没有的是空的。
    pub reason: Option<String>,
}

/// 记录 `records`（旧的在前）里 `period` 里的禁言、踢人，照人汇总、排好（此刻是 `now`）。只有解禁的人不算。
pub(crate) fn ranking(records: &[Record], period: Period, now: u64) -> Vec<Tally> {
    let since = period.since(now);
    let mut tallies: Vec<Tally> = Vec::new();
    for record in records.iter().filter(|record| record.at >= since) {
        if matches!(record.action, Action::Unmute | Action::Join) {
            continue;
        }
        let index = match tallies.iter().position(|tally| tally.user == record.user) {
            Some(index) => index,
            None => {
                tallies.push(Tally {
                    user: record.user,
                    name: None,
                    mutes: 0,
                    seconds: 0,
                    kicks: 0,
                    reason: None,
                });
                tallies.len() - 1
            }
        };
        let tally = &mut tallies[index];
        match record.action {
            Action::Mute => {
                tally.mutes += 1;
                tally.seconds += record.seconds.unwrap_or_default();
            }
            Action::Kick => tally.kicks += 1,
            Action::Unmute | Action::Join => {}
        }
        if record.name.is_some() {
            tally.name.clone_from(&record.name);
        }
        if record.reason.is_some() {
            tally.reason.clone_from(&record.reason);
        }
    }
    tallies.sort_by(|a, b| {
        (b.mutes, b.seconds, b.kicks)
            .cmp(&(a.mutes, a.seconds, a.kicks))
            .then(a.user.cmp(&b.user))
    });
    tallies
}

#[cfg(test)]
mod tests;
