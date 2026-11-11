//! 谁挂在哪的那张表，和它跟着总线上的名字走的先后（CI 上 macOS 抓到过的竞态）：订阅的回应（订阅那一刻的整份）在小任务里办，推来的
//! `bus.changed` 在桥的主循环里办，两条路的先后不一定。回应晚于一条推送办完的，照回应换上会把表盖回订阅那一刻的样子；总线以后
//! 不再变，刚起来时打开的插件就一直挂不上。
//!
//! 办法：每一次订阅发出去时领一张号（[`Followed::subscribing`]）；有订阅没回的时候，推来的那一份照「发出去的第几次订阅以后办的」
//! 记下来（推的是整份，只留最后一份）。回应到了（[`Followed::learn`]）：比已经换上的旧的不要；这次订阅发出去以后推来过的，照
//! 推来的那一份（它晚于回应的那一刻），不然照回应。订阅发出去以后推来的都晚于那一刻的整份（核心先答订阅、再推之后的变化；
//! `resync` 以后旧的订阅不再推）。施工 I-212 以前跟的是软件包列表，推的是一项一项的；总线推的是整份。

use std::collections::BTreeMap;

use serde_json::Value;

use super::table;

/// 那张表（包的编号到它挂的几处）和还没对上的订阅。
#[derive(Debug, Default)]
pub(super) struct Followed {
    /// 包的编号到它登记的挂接点，照编号排。
    pub(super) hooked: BTreeMap<String, Vec<&'static str>>,
    /// 发出去过几次订阅。
    sent: u64,
    /// 换上的是第几次订阅的回应。
    learned: u64,
    /// 有订阅没回时推来的最后一份：办它的时候发出去过几次订阅、那一份名字。
    pub(super) pushed: Option<(u64, Value)>,
}

impl Followed {
    /// 一次订阅要发出去了：领它的号。
    pub(super) fn subscribing(&mut self) -> u64 {
        self.sent += 1;
        self.sent
    }

    /// 第 `ticket` 次订阅的回应 `names`：比换上的旧的不要（交回假）；这次订阅以后推来过的照推来的那一份，不然照回应换上。
    pub(super) fn learn(&mut self, ticket: u64, names: &Value) -> bool {
        if ticket <= self.learned {
            return false;
        }
        self.hooked = match &self.pushed {
            Some((at, later)) if *at >= ticket => table(later),
            _ => table(names),
        };
        self.learned = ticket;
        if self.pushed.as_ref().is_some_and(|(at, _)| *at <= ticket) {
            self.pushed = None;
        }
        true
    }

    /// 推来的一份：照它换；有订阅没回的另记下来。
    pub(super) fn changed(&mut self, names: &Value) {
        self.hooked = table(names);
        if self.sent > self.learned {
            self.pushed = Some((self.sent, names.clone()));
        }
    }
}

#[cfg(test)]
mod tests;
