//! Telegram 能贴的表情（施工 I-185 下上，会话浮窗「确认主动回复后贴表情」旁边的列表）：Telegram 只认它给的那一套 Unicode 表情
//! （Bot API 的 `ReactionTypeEmoji`），编号就是那个字。从里面挑常用的几个；出厂贴 👀（同 QQ 的 289「睁眼」：她看到了、在回）。

use miyu_im::driver::Reaction;

/// 一个表情：编号和画出来的都是那个字。
const fn emoji(glyph: &'static str, name: &'static str) -> Reaction {
    Reaction {
        id: glyph,
        name,
        glyph: Some(glyph),
    }
}

/// 出厂贴的那一个。
pub const SEEN: &str = "👀";

/// 列表，照这个先后画。
pub const REACTIONS: &[Reaction] = &[
    emoji(SEEN, "看到了"),
    emoji("👍", "赞"),
    emoji("❤", "爱心"),
    emoji("🔥", "火"),
    emoji("👌", "好的"),
    emoji("🙏", "谢谢"),
    emoji("🎉", "庆祝"),
    emoji("😁", "笑"),
    emoji("🤔", "想想"),
    emoji("😢", "难过"),
];
