//! QQ 能贴的表情（施工 I-7 中三下二，会话浮窗「确认主动回复后贴表情」旁边的列表）：照 QQ 开放平台的表情对象那张表挑常用的几个。
//! 系统表情只有编号和名字（图标是 QQ 自己的，页面画不出来）；Unicode 表情的编号是码点的十进制，带上画得出来的那个字。出厂贴的是
//! 系统表情 289（睁眼，`bridge.json` 的 `reaction_emoji`），排在第一个。

use miyu_im::driver::Reaction;

/// 一个系统表情。
const fn face(id: &'static str, name: &'static str) -> Reaction {
    Reaction {
        id,
        name,
        glyph: None,
    }
}

/// 一个 Unicode 表情。
const fn emoji(id: &'static str, name: &'static str, glyph: &'static str) -> Reaction {
    Reaction {
        id,
        name,
        glyph: Some(glyph),
    }
}

/// 列表，照这个先后画。
pub(crate) const REACTIONS: &[Reaction] = &[
    face("289", "睁眼"),
    face("76", "赞"),
    face("201", "点赞"),
    face("66", "爱心"),
    face("319", "比心"),
    face("14", "微笑"),
    face("124", "OK"),
    face("269", "暗中观察"),
    face("271", "吃瓜"),
    face("282", "敬礼"),
    face("297", "拜谢"),
    face("299", "牛啊"),
    face("315", "加油"),
    face("320", "庆祝"),
    face("63", "玫瑰"),
    face("182", "笑哭"),
    face("264", "捂脸"),
    face("307", "喵喵"),
    face("32", "疑问"),
    emoji("128077", "厉害", "👍"),
    emoji("128079", "鼓掌", "👏"),
    emoji("128293", "火", "🔥"),
    emoji("10024", "闪光", "✨"),
    emoji("127881", "庆祝", "🎉"),
    emoji("128147", "爱心", "💓"),
    emoji("128516", "高兴", "😄"),
    emoji("10068", "问号", "❔"),
];

#[cfg(test)]
mod tests;
