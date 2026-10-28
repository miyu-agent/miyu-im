//! 引用和 @（`docs/blueprint/chat.md` 第五条「怎么走」第 4 条，`18-通讯平台.md` 第八节）：交进来的 [`Target`] 是「本来
//! 想要」，外面照这一轮的 `reply-to` 定好（施工时定的第 3 条）；这里看她回的那条之后群里的动静（[`Since`]），决定这一条
//! 实际带不带。
//!
//! 群里刚说完就回，不用引用也不用 @，带了反而吵；隔了几条别人的消息，读的人分不清她在回谁，才引用；隔了一阵、别人也说过
//! 话，那个人可能走开了，才 @。她连着说话时最后一条是她自己的，回下一个人要靠引用分清。私聊交进来两样都是假。智能引用、智能
//! 艾特关了的（`quote`、`mention`，施工 I-7 中三下二）从不带。
//!
//! [`Since`]: super::Since

use super::{OutCtx, OutStep, Outbound, OutboundRule, Outgoing, Since, Target};

/// 引用和 @ 这条规则。
pub(super) struct Rule;

impl OutboundRule for Rule {
    fn judge(&self, outgoing: Outgoing, target: Target, ctx: &OutCtx) -> OutStep {
        OutStep::Continue {
            outgoing,
            target: aimed(target, &ctx.since, &ctx.outbound),
        }
    }
}

/// 本来想要 `target`，她回的那条之后群里的动静是 `since`，参数是 `params`：实际带不带引用、@（这条规则本身；施工 I-64 起
/// 桥在交给平台的那一刻照那时的动静再算一次，打字停顿、排队这几秒里群里又来了话、她前一句先发了出去）。
pub fn aimed(target: Target, since: &Since, params: &Outbound) -> Target {
    // 图纸写的「`quote_after` 是 0」不另写一条：`others` 是无符号的，不少于 0 恒成立。
    let quote =
        target.quote && params.quote && (since.last_is_own || since.others >= params.quote_after);
    let mention = target.mention
        && params.mention
        && since.elapsed >= params.mention_after
        && since.others > 0;
    Target { quote, mention }
}
