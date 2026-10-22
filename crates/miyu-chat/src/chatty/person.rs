//! 按人冷静（施工 I-29，`docs/blueprint/chat.md` 第三条「怎么走」第 11 条，2026-10-23 项目主人定）：同一个人冲她来得勤，抬门槛。
//! 旧 Miyu 主群 3 天：她每天回九百多次，冲她来的占九成六，判官给的原始分中位已经过了门槛，减加分只少回 2%；同一个人一天叫她
//! 七十多次的也有。真人不会每句都接：头两三句你来我往照常，连着叫十几次就冷下来。
//!
//! 这个人最近被她回过的每一轮记一笔（她回的一轮里 `to` 有他），照半衰期衰减，加起来是 p（[`person_pressure`]）；p 不超过
//! `start` 不抬，往上照直线抬，到 `full` 抬满 `cap`。只管冲她来的（条件里有 `Direct`）：插嘴归冷静管。

use miyu_kernel::id::ExternalId;

use crate::Clock;

use super::{Chatty, Kind, Lift, LiftCtx, Reply};

/// `sender` 最近被她回过几轮：`replies` 里不晚于此刻、`to` 有他的每一轮加 `0.5^((now − at) ÷ half_life)`。先后不要紧。不看
/// 开没开：开关在 [`Lift`] 那一层看。
pub fn person_pressure(
    sender: &ExternalId,
    replies: &[Reply],
    clock: Clock,
    chatty: &Chatty,
) -> f64 {
    // 毫秒换成 f64：差出 2⁵³ 毫秒才丢精度，用不着管。
    let half_life = chatty.person.half_life as f64;
    replies
        .iter()
        .filter(|reply| reply.at <= clock.now && reply.to.contains(sender))
        .map(|reply| {
            let age = (clock.now.unix_millis() - reply.at.unix_millis()) as f64;
            0.5_f64.powf(age / half_life)
        })
        .sum()
}

/// 按人冷静这个修正。
pub(super) struct Item;

impl Lift for Item {
    fn lift(&self, ctx: &LiftCtx<'_>) -> f64 {
        let person = ctx.chatty.person;
        if !person.on || !ctx.conditions.has(Kind::Direct) {
            return 0.0;
        }
        let p = person_pressure(ctx.sender, ctx.replies, ctx.clock, ctx.chatty);
        if p <= person.start {
            return 0.0;
        }
        // `full` 不大于 `start` 的（参数写反了）一过 `start` 就抬满，不除以 0。
        let span = person.full - person.start;
        match span > 0.0 {
            true => person.cap * ((p - person.start) / span).min(1.0),
            false => person.cap,
        }
    }
}

#[cfg(test)]
mod tests;
