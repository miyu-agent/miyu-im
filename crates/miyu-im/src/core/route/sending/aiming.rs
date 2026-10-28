//! 带不带引用、@ 照交给 NapCat 的那一刻定（施工 I-64，`onebot.md`「群里怎么叫她」第 9 条 `since`）：写好那一刻定的，
//! 打字停顿（I-49、I-58）、排队那几秒以后不准了：群里又来了话，她前一句先发了出去。

use miyu_chat::Venue;

use super::super::outbound::{Aimed, relead};
use super::super::{Peer, Route, applied};
use crate::TARGET;
use crate::chat::Lead;

impl Route {
    /// 交给 NapCat 以前：群 `group`（主线的会话编号）这一刻的动静照投影数，这个群此刻的参数，重定 `aimed` 带不带引用、@，
    /// 填进 `lead`，记一行 `reply aimed`。投影、钟、场所读不出的照写好那一刻定的不动。
    pub(in crate::core::route) fn relead_now(
        &mut self,
        peer: &Peer,
        group: Option<&str>,
        aimed: &Aimed,
        lead: &mut Lead,
    ) {
        let (Some(projection), Some(clock), Some(venue)) = (
            group.and_then(|main| self.groups.get(main)),
            applied::clock(),
            Venue::parse(&peer.venue),
        ) else {
            return;
        };
        let (others, last_is_own) = projection.since_aim(aimed.aim.seq, &aimed.aim.sender);
        let outbound = self.applied(&venue).params.outbound;
        let target = relead(aimed, (others, last_is_own, clock.now), &outbound, lead);
        tracing::info!(target: TARGET, venue = %peer.venue, quote = target.quote, mention = target.mention, others, last_is_own, "reply aimed");
    }
}
