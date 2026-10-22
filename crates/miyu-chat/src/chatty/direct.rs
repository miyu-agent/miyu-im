//! 冲她来（`docs/blueprint/chat.md` 第三条「怎么走」第 2 条，18 第七节）：私聊里的每一条，群里 @ 她、以触发词（引用她不算，施工 I-20）
//! 开头的，由外面用 [`addressed()`](crate::addressed()) 算好交进来（[`Said::addressed`](crate::Said::addressed)）。加
//! `direct` 分，免冷静（免冷静写在冷静里，施工时定的第 3 条）。只叫了她、没说别的（[`Facts::hollow`](crate::Facts::hollow)，
//! 施工 I-29）照样成立，加 0 分。

use super::{Bonus, BonusCtx, Hit, Kind};

/// 冲她来这一项。
pub(super) struct Item;

impl Bonus for Item {
    fn judge(&self, ctx: &BonusCtx<'_>, _before: &[Hit]) -> Option<Hit> {
        ctx.facts.said.addressed.then_some(Hit {
            kind: Kind::Direct,
            bonus: match ctx.facts.hollow {
                true => 0.0,
                false => ctx.chatty.direct,
            },
        })
    }
}
