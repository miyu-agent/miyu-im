//! 刚说过话（`docs/blueprint/chat.md` 第三条「怎么走」第 4 条，18 第七节）：她最近说的一条（回谁都算）还在窗口里，任何人
//! 接着说。从这条消息以前她最后发出的那一条算起（施工 I-155），拆成几段发的照最后一段，不照这一轮的第一条。没有字的不算（只带了图、文件、表情，或者只有占位的，O-37）：发个图、贴个表情不是在接话。

use super::{Bonus, BonusCtx, Hit, Kind, last_said, within};

/// 刚说过话这一项。
pub(super) struct Item;

impl Bonus for Item {
    fn judge(&self, ctx: &BonusCtx<'_>, _before: &[Hit]) -> Option<Hit> {
        let window = ctx.chatty.after_speaking;
        let now = ctx.clock.now;
        let last = last_said(ctx.replies, now)?;
        let holds = !ctx.facts.textless && within(last, now, window.window);
        holds.then_some(Hit {
            kind: Kind::AfterSpeaking,
            bonus: window.bonus,
        })
    }
}
