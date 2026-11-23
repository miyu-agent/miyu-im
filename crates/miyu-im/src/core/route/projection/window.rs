//! 投影只留一个窗口（施工 LW-IM-7，`onebot.md` 第一条「群里怎么叫她」第 1 条）：人说的话（`said`）只留最近 `window_hours`
//! 小时、至少最近 `window_floor` 条，开过的回合（`turns`）只留最近 `window_hours` 小时，更早的丢掉。原来每条人话一直留着，
//! 一个热闹的群几万条，桥一直涨（2026-11-23 量过：每条约 330 字节）。
//!
//! 照窗口丢掉以后结果不变的理由：
//!
//! - 照时间数的（限流的回合、复读、续聊、刚说过话、顶替）窗口都不超过一天（[`miyu_chat::LONGEST_WINDOW`]），`window_hours` 读的
//!   时候查过不比它短（`tuning.rs`）；
//! - 照序号找的（判过要回的、正在跑的一轮回的那几条、看一眼交的没回过的、支线抄的）都是刚来的，最多 64 条，`window_floor` 不少于它；
//! - 她的回复（`replies`）、她发过的编号（`mine`）不丢：冷静照全部回复按半衰期加（半衰期最长一天，丢掉一天以前的会差出一截），
//!   个数跟她说了多少走，不跟群里的人话走；
//! - 扫全部人话的三样另记一张表（[`People`]），只跟群里说过话的人数走：这个人最近一次叫什么（`name_of`）、说过带 `owner` 的话
//!   没有（`is_admin`）、终端管理员用过的名字（`admin_names`）；
//! - 引用的那一条是谁发的（`origin`）：在窗口里的照旧认；丢掉了的路由另查聊天记录库（[`Projection::pruned`] 给查的时间范围，
//!   `core/route/quoted_sender.rs`），查不到的照「不知道是谁」。
//!
//! 起来时照旧从序号 0 补日志、照同样的规则丢，表跟着建回来，不落盘。

use std::collections::HashMap;
use std::time::Duration;

use miyu_client::ids::ExternalId;
use miyu_client::time::Timestamp;

use super::{Projection, Speaker};

/// 留多久、至少留几条。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::core::route) struct Keep {
    /// 留最近多久，毫秒。
    pub(in crate::core::route) millis: i64,
    /// 人说的话至少留最近几条。
    pub(in crate::core::route) floor: usize,
}

impl Keep {
    /// 照 `bridge.json` 的 `window_hours`、`window_floor`（`Tuning::window`）。
    pub(in crate::core::route) fn new((keep, floor): (Duration, usize)) -> Keep {
        Keep {
            millis: i64::try_from(keep.as_millis()).unwrap_or(i64::MAX),
            floor,
        }
    }
}

/// 群里说过话的一个人：最近一次带名字的那一条叫什么、说过带 `owner` 的话没有。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Person {
    /// 最近一次写了名字的那一条的名字。
    name: Option<String>,
    /// 说过终端管理员的话没有（`by.owner`）。
    admin: bool,
}

/// 扫全部人话的三样（模块的说明）：大小跟说过话的人数走，不丢。
#[derive(Debug, Default)]
pub(super) struct People {
    /// 平台身份 → 这个人。
    people: HashMap<ExternalId, Person>,
    /// 终端管理员用过的名字，去重，照第一次的先后。
    admin_names: Vec<String>,
    /// 留多久；没给的（测试里直接造的投影）一条都不丢。
    keep: Option<Keep>,
    /// 收到的第一条人话的时刻：查记录库时不早于它（导进来的更早的聊天记录不算这个会话的）。
    first: Option<Timestamp>,
    /// 丢掉的人话里最晚的那一条的时刻；一条都没丢的是空的。
    pruned: Option<Timestamp>,
}

impl Projection {
    /// 照 `keep` 只留一个窗口（路由造投影时给；测试里直接造的不给，一条都不丢）。
    pub(in crate::core::route) fn windowed(mut self, keep: Keep) -> Projection {
        self.people.keep = Some(keep);
        self
    }

    /// 收了一条人话 `speaker`：记进表里（`take` 在插进 `said` 以前调，插进去以后再调 [`Self::drop_old`]）。
    pub(super) fn heard_from(&mut self, speaker: &Speaker) {
        let people = &mut self.people;
        people.first.get_or_insert(speaker.at);
        let person = people.people.entry(speaker.id.clone()).or_default();
        person.admin |= speaker.admin;
        if let Some(name) = &speaker.name {
            person.name = Some(name.clone());
            if speaker.admin && !people.admin_names.contains(name) {
                people.admin_names.push(name.clone());
            }
        }
    }

    /// 照窗口丢旧的（模块的说明）：此刻是 `now`。
    pub(super) fn drop_old(&mut self, now: Timestamp) {
        let Some(keep) = self.people.keep else {
            return;
        };
        let cutoff = now.unix_millis().saturating_sub(keep.millis);
        while self.said.len() > keep.floor
            && let Some(entry) = self.said.first_entry()
            && entry.get().at.unix_millis() < cutoff
        {
            let gone = entry.remove();
            self.people.pruned = Some(self.people.pruned.map_or(gone.at, |at| at.max(gone.at)));
        }
        if self
            .turns
            .first()
            .is_some_and(|turn| turn.at.unix_millis() < cutoff)
        {
            self.turns.retain(|turn| turn.at.unix_millis() >= cutoff);
        }
    }

    /// 丢掉过人话的：交回查记录库的时间范围，从收到的第一条到丢掉的最晚的那一条（`origin` 在窗口里找不到引用的那一条时，路由
    /// 照它查）。一条都没丢的是空的：在窗口里找不到的就是不知道。
    pub(in crate::core::route) fn pruned(&self) -> Option<(Timestamp, Timestamp)> {
        Some((self.people.first?, self.people.pruned?))
    }

    /// 平台上的 `who` 是不是终端管理员：他在这个群说过的话带 `owner`（「施工时定的」第 183 条；命令被拒的那一句听不听得到照它
    /// 兜底，O-47）。
    pub(in crate::core::route) fn is_admin(&self, who: &ExternalId) -> bool {
        self.people
            .people
            .get(who)
            .is_some_and(|person| person.admin)
    }

    /// 平台上的 `who` 在这个群最近一次说话时叫什么（施工 S-1 上：`chat.venue` 交给插件的名字）；没说过、没写名字的是空的。
    pub(in crate::core::route) fn name_of(&self, who: &ExternalId) -> Option<String> {
        self.people.people.get(who)?.name.clone()
    }

    /// 说过话、写过名字的人各自最近的名字（施工 LW-IM-7：`chat.venue` 晚一点答时照它给引用的那个人起名字，同 [`Self::name_of`]）。
    pub(in crate::core::route) fn known_names(&self) -> HashMap<ExternalId, String> {
        self.people
            .people
            .iter()
            .filter_map(|(id, person)| Some((id.clone(), person.name.clone()?)))
            .collect()
    }

    /// 终端管理员在这个群用过的名字（O-53：冒充提醒照它比），去重，照第一次出现的先后。
    pub(in crate::core::route) fn admin_names(&self) -> Vec<String> {
        self.people.admin_names.clone()
    }
}

#[cfg(test)]
mod tests;
