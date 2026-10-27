//! 按群冷静（施工 I-62，`docs/blueprint/chat.md` 第三条「怎么走」第 14 条，2026-10-27 项目主人看了重放选的档）：她在这个场所
//! 最近回得多，冲她来的门槛一起抬。10-27 晚主群一晚四个小时她发了四百多条，每十分钟中位 21 条，大多是好几个人同时叫她；按人
//! 冷静只压叫得勤的那一两个人，压不住人多。
//!
//! 她在这个场所回过的每一轮记一笔（不分回的是谁），照半衰期衰减，加起来是 p；p 不超过 `start` 不抬，往上照直线抬，到 `full`
//! 抬满 `cap`（同按人冷静，[`super::person::ramp`]）。只管冲她来的（条件里有 `Direct`）：插嘴归冷静管。

use super::person::ramp;
use super::{Kind, Lift, LiftCtx};

/// 按群冷静这个修正。
pub(super) struct Item;

impl Lift for Item {
    fn lift(&self, ctx: &LiftCtx<'_>) -> f64 {
        let group = ctx.chatty.group;
        if !group.on || !ctx.conditions.has(Kind::Direct) {
            return 0.0;
        }
        // 毫秒换成 f64：差出 2⁵³ 毫秒才丢精度，用不着管。
        let half_life = group.half_life as f64;
        let p: f64 = ctx
            .replies
            .iter()
            .filter(|reply| reply.at <= ctx.clock.now)
            .map(|reply| {
                let age = (ctx.clock.now.unix_millis() - reply.at.unix_millis()) as f64;
                0.5_f64.powf(age / half_life)
            })
            .sum();
        ramp(p, &group)
    }
}

#[cfg(test)]
mod tests;
