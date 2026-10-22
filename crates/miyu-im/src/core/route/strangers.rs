//! 陌生人的私聊（施工 O-60，`onebot.md`「陌生人的私聊」）：不是终端管理员、不在白名单里的人私聊机器人号，照「其他私聊」卡办。
//!
//! - 一条照群里的进站链判（`miyu_chat::Chain::builtin`，发的人是普通成员，私聊一律冲她来）：卡片「回复」关着（`not_allowed`）的
//!   照原来不接；睡觉时间、限流满了的只记下、不开回合；放行的开一轮。
//! - 限流数开过的回合：陌生人的私聊没有投影，开过的时刻记在桥的内存里（按场所，只留一天以内的），桥重启从空的开始。

use std::collections::HashMap;

use miyu_chat::{Chain, Ctx, Inbound, Moderation, Outcome, Said, Standing, Venue, VenueKind};
use miyu_client::ids::ExternalId;
use miyu_client::time::Timestamp;

use super::{Route, applied};

/// 一天，毫秒：限流的窗口最长一天，再早的不用留。
const DAY: i64 = 24 * 60 * 60 * 1000;

/// 陌生人的私聊开过的回合：场所编号的原文 → 开的时刻。
#[derive(Debug, Default)]
pub(super) struct Strangers {
    turns: HashMap<String, Vec<Timestamp>>,
}

impl Strangers {
    /// 场所 `venue` 此刻 `now` 开了一个回合：记下，顺手扔掉一天以前的。
    pub(super) fn opened(&mut self, venue: &str, now: Timestamp) {
        let turns = self.turns.entry(venue.to_string()).or_default();
        turns.retain(|turn| now.unix_millis() - turn.unix_millis() < DAY);
        turns.push(now);
    }

    /// 场所 `venue` 开过的回合。
    fn of(&self, venue: &str) -> Vec<Timestamp> {
        self.turns.get(venue).cloned().unwrap_or_default()
    }
}

/// 私聊过进站链的是谁：陌生人（谁能叫她、限流、睡眠都管），白名单成员卡上关了忽略限流、忽略睡眠的（施工 I-7 中三下一：只管关了
/// 的那一样，`rate`、`sleep` 是管不管）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Gating {
    /// 陌生人。
    Stranger,
    /// 白名单成员：管不管限流、睡眠。
    Listed { rate: bool, sleep: bool },
}

impl Route {
    /// 陌生人 `sender` 在私聊 `venue` 说的一条 `text` 过进站链：交回结果（本机的钟读不出的，照说不会，当不让叫她）。放行的记下
    /// 这一刻开了一个回合。
    pub(super) fn stranger(&mut self, venue: &Venue, sender: &ExternalId, text: &str) -> Outcome {
        self.gated(venue, sender, text, Gating::Stranger)
    }

    /// 同 [`Route::stranger`]，照 `gating` 管哪几样（白名单成员的回合和陌生人的一样数，施工 I-7 中三下一）。
    pub(super) fn gated(
        &mut self,
        venue: &Venue,
        sender: &ExternalId,
        text: &str,
        gating: Gating,
    ) -> Outcome {
        let Some(clock) = applied::clock() else {
            return Outcome::RecordOnly(miyu_chat::Why::NotAllowed);
        };
        let loaded = self.rules.current(std::time::Instant::now());
        let applied = loaded.at(venue);
        let (rate, sleep, allow) = match gating {
            Gating::Stranger => (true, true, applied::allow(&applied)),
            Gating::Listed { rate, sleep } => (rate, sleep, None),
        };
        let ctx = Ctx {
            rate: applied::rate(&applied).filter(|_| rate),
            sleep: applied::sleep(&applied).filter(|_| sleep),
            allow,
            muted: false,
            repeated: false,
            turns: self.strangers.of(venue.id().as_str()),
            moderation: Moderation {
                keywords: loaded.keywords.clone(),
                base64: applied.params.base64,
            },
        };
        let msg = Inbound {
            kind: VenueKind::Private,
            said: Said {
                sender: sender.clone(),
                standing: Standing::Member,
                addressed: true,
            },
            text: text.to_string(),
        };
        let outcome = Chain::builtin().judge(&msg, &ctx, clock).outcome;
        if outcome == Outcome::Pass {
            self.strangers.opened(venue.id().as_str(), clock.now);
        }
        outcome
    }
}

#[cfg(test)]
mod tests;
