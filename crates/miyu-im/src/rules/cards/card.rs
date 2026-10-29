//! 一张卡是哪一张（从 `cards.rs` 挪来，施工 I-80 带上平台）：拼出来的文件名、交给页面的写法；号拼进文件名、规则的写法（施工 I-79）。
//!
//! 文件名：QQ 的照旧 `70-card-a.toml`、`70-card-b.toml`、`70-card-g-<群号>.toml`、`70-card-p-<号>.toml`；别的平台的在前面加
//! `<平台>~`（`70-card-tg~g-<群号>.toml`）。`~` 不在号的写法里（[`file_part`]），也不在平台名里，认得回来。

use serde_json::{Value, json};

use miyu_chat::{Venue, VenueKind};

use super::PREFIX;
use crate::chat::Id;

/// 卡片上没写平台的算它：I-80 以前存的卡片都是 OneBot 驱动的。也是文件名里不带平台的那一个。
pub const OLD_PLATFORM: &str = "qq";

/// 文件名里平台和后面隔开的字。
const AFTER_PLATFORM: char = '~';

/// 一张卡是哪一张：拼出来的文件名里写着。平台是驱动报的名字。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Card {
    /// 会话默认值（施工 I-7 下二）：不分平台。
    Defaults,
    /// 这个平台的「其他群」。
    OtherGroups(String),
    /// 这个平台的「其他私聊」（施工 O-60）。
    OtherPrivates(String),
    /// 这个平台上群号是它的那个群。
    Group(String, Id),
    /// 这个平台上对方的号是它的那个私聊。
    Private(String, Id),
}

impl Card {
    /// 是哪个平台的：会话默认值不分平台，是空的。
    pub fn platform(&self) -> Option<&str> {
        match self {
            Card::Defaults => None,
            Card::OtherGroups(platform)
            | Card::OtherPrivates(platform)
            | Card::Group(platform, _)
            | Card::Private(platform, _) => Some(platform),
        }
    }

    /// 拼出来的文件名（见模块开头）：照字排，同一个平台的是「其他群」、「其他私聊」、群、私聊。
    pub(in crate::rules) fn file_name(&self) -> String {
        let head = match self.platform() {
            Some(platform) if platform != OLD_PLATFORM => format!("{platform}{AFTER_PLATFORM}"),
            _ => String::new(),
        };
        let rest = match self {
            Card::Defaults => "0".to_string(),
            Card::OtherGroups(_) => "a".to_string(),
            Card::OtherPrivates(_) => "b".to_string(),
            Card::Group(_, id) => format!("g-{}", file_part(id)),
            Card::Private(_, id) => format!("p-{}", file_part(id)),
        };
        format!("{PREFIX}{head}{rest}.toml")
    }

    /// 照文件名认是哪一张卡：不是卡片拼的文件的是空的。
    pub fn parse(file: &str) -> Option<Card> {
        let rest = file.strip_prefix(PREFIX)?.strip_suffix(".toml")?;
        let (platform, rest) = match rest.split_once(AFTER_PLATFORM) {
            Some((platform, rest)) if platform_ok(platform) && platform != OLD_PLATFORM => {
                (platform.to_string(), rest)
            }
            Some(_) => return None,
            None if rest == "0" => return Some(Card::Defaults),
            None => (OLD_PLATFORM.to_string(), rest),
        };
        match rest {
            "a" => return Some(Card::OtherGroups(platform)),
            "b" => return Some(Card::OtherPrivates(platform)),
            _ => {}
        }
        let (kind, id) = rest.split_once('-')?;
        let id = from_file_part(id)?;
        match kind {
            "g" => Some(Card::Group(platform, id)),
            "p" => Some(Card::Private(platform, id)),
            _ => None,
        }
    }

    /// 交给页面的写法：`{"kind": "defaults"}`、`{"kind": "other_groups", "platform": …}`、`{"kind": "group", "platform": …,
    /// "id": …}`……
    pub fn json(&self) -> Value {
        match self {
            Card::Defaults => json!({"kind": "defaults"}),
            Card::OtherGroups(platform) => json!({"kind": "other_groups", "platform": platform}),
            Card::OtherPrivates(platform) => {
                json!({"kind": "other_privates", "platform": platform})
            }
            Card::Group(platform, id) => json!({"kind": "group", "platform": platform, "id": id}),
            Card::Private(platform, id) => {
                json!({"kind": "private", "platform": platform, "id": id})
            }
        }
    }
}

/// 平台名合不合写法：照群聊内核拼场所编号的规矩查（施工 I-80：卡片上写的平台拼进文件名、规则）。
pub fn platform_ok(platform: &str) -> bool {
    Venue::new(platform, VenueKind::Group, "1").is_ok()
}

/// 号拼进规则的写法（施工 I-79）：十进制整数照数写（只用 QQ 的人拼出来的文件不变），别的写成字（照 JSON 的转义，TOML 基本字符串
/// 都认）。
pub(in crate::rules) fn toml_id(id: &Id) -> String {
    match id.number() {
        Some(number) => number.to_string(),
        None => toml_text(id.as_str()),
    }
}

/// 字拼进规则的写法：照 JSON 的转义（TOML 基本字符串都认）。
pub(in crate::rules) fn toml_text(text: &str) -> String {
    Value::String(text.to_string()).to_string()
}

/// 号拼进文件名的写法（施工 I-79）：字母、数字和 `_`、`-`、`.`、`@` 照写，别的每个字节写成 `%XX`（大写）。QQ 的号照原样。
fn file_part(id: &Id) -> String {
    let mut part = String::new();
    for byte in id.as_str().bytes() {
        if byte.is_ascii_alphanumeric() || b"_-.@".contains(&byte) {
            part.push(char::from(byte));
        } else {
            part.push_str(&format!("%{byte:02X}"));
        }
    }
    part
}

/// [`file_part`] 的反面：写法不对、解出来不是号的是空的。
fn from_file_part(part: &str) -> Option<Id> {
    let mut bytes = Vec::new();
    let mut rest = part.as_bytes();
    while let Some((&byte, after)) = rest.split_first() {
        if byte == b'%' {
            let hex = std::str::from_utf8(after.get(..2)?).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            rest = &after[2..];
        } else {
            bytes.push(byte);
            rest = after;
        }
    }
    Id::new(&String::from_utf8(bytes).ok()?)
}
