//! 时不时看一眼（设计 05，施工 I-89）的纯逻辑：定时里存什么（[`Prompt`]）、起来时和卡片变了时照哪几个场所对账（[`reconcile`]）、
//! 到点该交哪几条新消息（[`unanswered`]）。计时用核心的定时（`schedule.*`，一次性的 `at`，到点推 `schedule.fired`）；
//! im 不自己计时，接线在 `crate::core::route` 的 `glance`。
//!
//! - 定时的 `prompt` 是一份 JSON：哪个场所、上次看到哪一条（主线序号）、间隔几秒。到点原样交回，im 不另外记状态。
//! - 间隔没定的照 `bridge.json` 的 `glance_seconds`（出厂 7 分钟，施工 I-143；原来 25 分钟，设计 05 第一节第 2 条）；她自己定下一次的工具随施工 I-89 下。

use serde::{Deserialize, Serialize};

/// 一次交给 `session.respond` 的新消息最多几条（核心的 `to` 最多 64 条）。
pub const MOST: usize = 64;

/// 写进定时的那一份：到点原样交回来。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prompt {
    /// 场所编号的原文。
    pub venue: String,
    /// 上次看到哪一条：主线序号，晚于它的才算新的；没看过的是建定时那一刻收到的最后一条。
    pub after: u64,
    /// 这一次的间隔，秒：没有新消息的照它再排一次。
    pub seconds: u64,
    /// 是她定的（`qq_glance`，施工 I-114：会话浮窗「概况」、运行日志写清是谁定的）。I-114 以前的定时没有这一格，当不是。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hers: bool,
}

impl Prompt {
    /// 写成定时的 `prompt`。
    #[must_use]
    pub fn text(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// 读定时交回来的 `prompt`；不是这个样子的（别的写法、手造的）是空的。
    #[must_use]
    pub fn read(text: &str) -> Option<Prompt> {
        serde_json::from_str(text).ok()
    }
}

/// 对账的结果：删哪几个定时、给哪几个场所新建。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Reconciled {
    /// 要删的定时编号：认得的场所不该看了的、读不出的、同一个场所多出来的。
    pub cancel: Vec<String>,
    /// 要新建的场所：该看却没有定时的，照 `want` 的先后。
    pub add: Vec<String>,
}

/// 照认得的场所 `known`（这时有会话的）、其中该看的 `want` 和已有的定时 `have`（编号、读出的 `prompt`，读不出的是空的）对账：
/// 每个场所留头一个定时，多的删掉；认得、不该看的删掉；不认得的场所留着（列会话没成、还没来过消息的不误删，到点再照开关查）；
/// 该看却没有的新建。
#[must_use]
pub fn reconcile(
    (want, known): (&[String], &[String]),
    have: &[(String, Option<Prompt>)],
) -> Reconciled {
    let mut kept: Vec<&str> = Vec::new();
    let mut done = Reconciled::default();
    for (id, prompt) in have {
        match prompt {
            Some(prompt)
                if (want.contains(&prompt.venue) || !known.contains(&prompt.venue))
                    && !kept.contains(&prompt.venue.as_str()) =>
            {
                kept.push(&prompt.venue);
            }
            _ => done.cancel.push(id.clone()),
        }
    }
    done.add = want
        .iter()
        .filter(|venue| !kept.contains(&venue.as_str()))
        .cloned()
        .collect();
    done
}

/// 到点该交的新消息：`said` 是人说的话（主线序号、当过一轮的触发没有），晚于 `after`、没当过触发的才算（当过的她已经回过或者
/// 在回，核心也不让再当）；照先后，多了留最后 `most` 条（卡上「看一眼带几条」，出厂 50，施工 I-175），最多 [`MOST`] 条。
#[must_use]
pub fn unanswered(said: impl Iterator<Item = (u64, bool)>, after: u64, most: usize) -> Vec<u64> {
    let fresh: Vec<u64> = said
        .filter(|(seq, answered)| *seq > after && !answered)
        .map(|(seq, _)| seq)
        .collect();
    let skip = fresh.len().saturating_sub(most.clamp(1, MOST));
    fresh.into_iter().skip(skip).collect()
}

#[cfg(test)]
mod tests;
