//! 刚说过话（`docs/blueprint/chat.md` 第三条「怎么走」第 4 条，18 第七节）：她最近说的一条（回谁都算）还在窗口里，任何人
//! 接着说。从这条消息以前她最后发出的那一条算起（施工 I-155），拆成几段发的照最后一段，不照这一轮的第一条。只发图、表情的也算（施工 I-164，项目主人 11-05：她刚说完话有人回个图，多半是在接她；O-37 起到 I-163 是不算的）。

use super::{Bonus, BonusCtx, Hit, Kind, last_said, within};

/// 刚说过话这一项。
pub(super) struct Item;

impl Bonus for Item {
    fn judge(&self, ctx: &BonusCtx<'_>, _before: &[Hit]) -> Option<Hit> {
        let window = ctx.chatty.after_speaking;
        let now = ctx.clock.now;
        let last = last_said(ctx.replies, now)?;
        let holds = within(last, now, window.window);
        holds.then_some(Hit {
            kind: Kind::AfterSpeaking,
            bonus: window.bonus,
        })
    }
}
