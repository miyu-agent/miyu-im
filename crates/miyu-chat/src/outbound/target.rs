//! 引用和 @（`docs/blueprint/chat.md` 第五条「怎么走」第 4 条，`18-通讯平台.md` 第八节）：交进来的 [`Target`] 是「本来
//! 想要」，外面照这一轮的 `reply-to` 定好（施工时定的第 3 条）；这里看她回的那条之后群里的动静（[`Since`]），决定这一条
//! 实际带不带。
//!
//! 群里刚说完就回，不用引用也不用 @，带了反而吵；中间插进了别的话（出厂 2 条，施工 I-137：照主群 10 天的真人数据，真人中间隔
//! 一两条就引用，项目主人定 2 条），读的人分不清她在回谁，才引用；她连着说话时最后一条是她自己的，回下一个人也要靠引用分清。@ 只跟着引用一起带
//! （施工 I-137：真人的 @ 九成多是引用时带上的，单独 @ 不引用的只占有对象的消息的约 6%，她原来是 41%）：引用了就 @（施工 I-216
//! 以前还要隔一阵，出厂 20 秒；她几秒就回，线上一天只有 4% 的回话带了 @，项目主人 11-11 定去掉）。
//! 智能引用关了的，@ 照原来的办：别人说过话、隔了一阵（`mention_after`，出厂 20 秒）才 @。私聊交进来两样都是假。智能引用、智能艾特关了的（`quote`、`mention`，
//! 施工 I-7 中三下二）从不带。
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
    // @ 跟着引用走（施工 I-137）：这一条引用了就一起 @，不再等「隔了一阵」（施工 I-216，项目主人 11-11：她现在几秒就回，
    // 等 20 秒的话引用了也几乎不 @）。智能引用关了的照原来：别人说过话、隔了一阵才 @。
    let along = match params.quote {
        true => quote,
        false => since.others > 0 && since.elapsed >= params.mention_after,
    };
    let mention = target.mention && params.mention && along;
    Target { quote, mention }
}
