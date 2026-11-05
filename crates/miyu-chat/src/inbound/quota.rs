//! 额度（im 设计 08「使用限制」，施工 I-135）：这个会话、发的人最近 5 小时、7 天、30 天的 token 用量碰到了上限没有，由外面照用量
//! 账本判好交进来（[`Quota`]），这里不数。满了的只记下，冲她来的也不出声（同限流，施工 O-47）。豁免同限流：终端管理员、白名单
//! 成员不受它管（私聊卡上关了「忽略限流」的，外面照限流那样把白名单成员当别的人交进来）。

use super::{Clock, Ctx, Inbound, InboundRule, Outcome, Standing, Step, Why};

/// 额度满了没有，外面判好交进来。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Quota {
    /// 这个会话（卡片上的三条线）满了：谁说的都只记下（豁免的人除外），线路规程也不调判官（[`quota_full`]）。
    pub venue: bool,
    /// 发这一条的人（使用限制页上的三条线）满了：只拦他的。
    pub person: bool,
}

/// 这个会话的额度满了没有：满了的这段时间，线路规程不调判官（同 [`super::rate_full`]）。不看发的人。
pub fn quota_full(ctx: &Ctx) -> bool {
    ctx.quota.venue
}

/// 额度这条规则。
pub(super) struct Rule;

impl InboundRule for Rule {
    fn judge(&self, msg: &Inbound, ctx: &Ctx, _clock: Clock) -> Step {
        if msg.said.standing != Standing::Member {
            return Step::Continue;
        }
        match ctx.quota.venue || ctx.quota.person {
            true => Step::Stop(Outcome::RecordOnly(Why::QuotaReached)),
            false => Step::Continue,
        }
    }
}

#[cfg(test)]
mod tests;
