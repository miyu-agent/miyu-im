//! 概率触发（设计 12 第一节第 1 条，施工 I-197；项目主人 2026-11-10 定）：开着的场所，别的条件一个都不成立的消息照概率抽，
//! 抽中的算一个条件。不加分：主动回复判断开着的照常交判官打分，关着的由线路规程直接开一轮、接不接话她定。冷静那几项不抬它的
//! 门槛（各项自己认）。和设计 05 删掉的抽样不是一回事：那一个只有判官开着才有、抽中的一定交判官。
//!
//! 不碰随机源（`02-内核.md` K1）：照场所编号和这条消息在日志里的序号算一个数跟千分比比，同一份日志回放结果一样。
//! 没有字的（只有图、表情、文件）照 `sample_skip_media` 不抽（出厂不抽）。

use sha2::{Digest, Sha256};

use super::{Bonus, BonusCtx, Hit, Kind};

/// 概率触发这一项：排在最后，前面有一项成立就不抽。
pub(super) struct Item;

impl Bonus for Item {
    fn judge(&self, ctx: &BonusCtx<'_>, before: &[Hit]) -> Option<Hit> {
        let sample = ctx.chatty.sample;
        if !sample.on || !before.is_empty() || (sample.skip_media && ctx.facts.media_only) {
            return None;
        }
        let drawn = draw(ctx.facts.venue.as_str(), ctx.facts.msg.get());
        (drawn < sample.rate).then_some(Hit {
            kind: Kind::Sampled,
            bonus: 0.0,
        })
    }
}

/// 场所 `venue` 的第 `msg` 条抽到的数：0 到 999。
fn draw(venue: &str, msg: u64) -> u32 {
    let mut hasher = Sha256::new();
    hasher.update(venue.as_bytes());
    hasher.update([0]);
    hasher.update(msg.to_be_bytes());
    let digest = hasher.finalize();
    u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) % 1000
}

#[cfg(test)]
mod tests;
