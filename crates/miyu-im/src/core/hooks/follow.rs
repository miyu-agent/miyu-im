//! 谁挂在哪的那张表，和它跟着软件包列表走的先后（CI 上 macOS 抓到过的竞态）：订阅的回应（订阅那一刻的整份）在小任务里办，推来的
//! `package.changed` 在桥的主循环里办，两条路的先后不一定。回应晚于一条推送办完的，整份换上会把推来的那一项盖回订阅那一刻的样子；
//! 那个包以后不再变，就一直挂不上（刚起来时打开的插件）。
//!
//! 办法：每一次订阅发出去时领一张号（[`Followed::subscribing`]）；有订阅没回的时候，推来的每一项照「发出去的第几次订阅以后办的」记
//! 下来。回应到了（[`Followed::learn`]）：比已经换上的旧的那一份不要；换上整份以后，把这次订阅发出去以后推来的几项照先后再办一遍。
//! 订阅发出去以后推来的都晚于那一刻的整份（核心先答订阅、再推之后的变化；`resync` 以后旧的订阅不再推）。

use std::collections::BTreeMap;

use serde_json::Value;

use super::{change, table};

/// 那张表（包的编号到它挂的几处）和还没对上的订阅。
#[derive(Debug, Default)]
pub(super) struct Followed {
    /// 包的编号到它导出的挂接点，照编号排。
    pub(super) hooked: BTreeMap<String, Vec<&'static str>>,
    /// 发出去过几次订阅。
    sent: u64,
    /// 换上的是第几次订阅的回应。
    learned: u64,
    /// 有订阅没回时推来的：办它的时候发出去过几次订阅、包、那一项。
    pushed: Vec<(u64, String, Value)>,
}

impl Followed {
    /// 一次订阅要发出去了：领它的号。
    pub(super) fn subscribing(&mut self) -> u64 {
        self.sent += 1;
        self.sent
    }

    /// 第 `ticket` 次订阅的回应 `packages`：比换上的旧的不要（交回假）；不然整份换上，再把这次订阅以后推来的照先后办一遍。
    pub(super) fn learn(&mut self, ticket: u64, packages: &Value) -> bool {
        if ticket <= self.learned {
            return false;
        }
        self.hooked = table(packages);
        for (_, package, entry) in self.pushed.iter().filter(|(at, ..)| *at >= ticket) {
            change(&mut self.hooked, package, entry);
        }
        self.learned = ticket;
        self.pushed.retain(|(at, ..)| *at > ticket);
        true
    }

    /// 推来的一项：照它换那一个包；有订阅没回的另记下来。
    pub(super) fn changed(&mut self, package: &str, entry: &Value) {
        change(&mut self.hooked, package, entry);
        if self.sent > self.learned {
            self.pushed
                .push((self.sent, package.to_string(), entry.clone()));
        }
    }
}

#[cfg(test)]
mod tests;
