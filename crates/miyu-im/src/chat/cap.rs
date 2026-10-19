//! 平台能力（施工 I-8 上，设计 01 第三节第 3、4 条）：一个平台做得到的几样。驱动报它的能力表（`Driver::caps`），宿主照它算
//! 插件在哪个平台上能用（插件声明的 `needs`）。名字是小写英文，插件的声明照它写。

/// 平台做得到的一样。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cap {
    /// 撤回一条消息。
    Recall,
    /// 禁言、解禁。
    Mute,
    /// 戳一戳。
    Poke,
    /// 在消息上贴表情。
    React,
    /// 发合并转发。
    Forward,
    /// 列群成员、问一个成员。
    Members,
    /// 取头像。
    Avatar,
    /// 取带的图、视频、文件。
    Media,
    /// 取一条消息。
    Message,
    /// 传文件。
    Upload,
    /// 同意加好友。
    Befriend,
    /// 原样调用平台的动作（`chat.action`）。
    Raw,
}

/// 每一样，照先后。
pub const CAPS: [Cap; 12] = [
    Cap::Recall,
    Cap::Mute,
    Cap::Poke,
    Cap::React,
    Cap::Forward,
    Cap::Members,
    Cap::Avatar,
    Cap::Media,
    Cap::Message,
    Cap::Upload,
    Cap::Befriend,
    Cap::Raw,
];

impl Cap {
    /// 名字：插件的声明照它写。
    pub fn name(self) -> &'static str {
        match self {
            Cap::Recall => "recall",
            Cap::Mute => "mute",
            Cap::Poke => "poke",
            Cap::React => "react",
            Cap::Forward => "forward",
            Cap::Members => "members",
            Cap::Avatar => "avatar",
            Cap::Media => "media",
            Cap::Message => "message",
            Cap::Upload => "upload",
            Cap::Befriend => "befriend",
            Cap::Raw => "raw",
        }
    }

    /// 照名字认；认不出的是空的。
    pub fn parse(name: &str) -> Option<Cap> {
        CAPS.into_iter().find(|cap| cap.name() == name)
    }
}

#[cfg(test)]
mod tests;
