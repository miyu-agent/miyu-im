//! 跟进（施工 I-21，2026-10-21 项目主人定「群聊三段：覆盖、跟进、续聊」）：她正在为这个人跑一轮（主线或者一条支线在回他），他又
//! 说了一条，像在终端里她回答时又追加了一句。照常判断，判下来要回的并进那一轮，她在步和步之间看到；只说一句、不调工具的那种轮，
//! 她答完再接着答。先照「这条线还在跑」算，不用固定的秒数（项目主人：「先试试」），开关是场所规则的 `chatty.followup`。同续聊：
//! @ 了别人、引用别人的消息的是对别人说的，不算；只有图的照算。

use super::{Bonus, BonusCtx, Hit, Kind};

/// 跟进这一项。
pub(super) struct Item;

impl Bonus for Item {
    fn judge(&self, ctx: &BonusCtx<'_>, _before: &[Hit]) -> Option<Hit> {
        let facts = ctx.facts;
        let followup = ctx.chatty.followup;
        let holds = followup.on
            && ctx.serving.contains(&facts.said.sender)
            && !facts.mentions_others
            && !facts.quotes_other;
        holds.then_some(Hit {
            kind: Kind::FollowUp,
            bonus: followup.bonus,
        })
    }
}

#[cfg(test)]
mod tests;
