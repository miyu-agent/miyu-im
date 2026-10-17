//! 去掉末尾的句号（`docs/blueprint/chat.md` 第五条「怎么走」第 8 条，施工 O-47，2026-10-17 项目主人定，照旧版）：聊天里一句话
//! 末尾带 `。` 显得生硬。只去一个，只去中文句号；末尾的空白不算，去掉以后整条空了的不去（只有一个 `。` 的照发）。场所规则
//! `[outbound] strip_period = false` 关掉。

use super::{OutCtx, OutStep, OutboundRule, Outgoing, Target};

/// 中文句号。
const PERIOD: char = '。';

/// 去句号这条规则。
pub(super) struct Rule;

impl OutboundRule for Rule {
    fn judge(&self, mut outgoing: Outgoing, target: Target, ctx: &OutCtx) -> OutStep {
        if ctx.outbound.strip_period {
            strip(&mut outgoing.text);
        }
        OutStep::Continue { outgoing, target }
    }
}

/// 去掉 `text` 末尾（不算空白）的一个 `。`；去了以后只剩空白的不去。
fn strip(text: &mut String) {
    let body = text.trim_end();
    let Some(kept) = body.strip_suffix(PERIOD) else {
        return;
    };
    if kept.trim().is_empty() {
        return;
    }
    text.truncate(kept.len());
}

#[cfg(test)]
mod tests;
