//! 一张卡是哪一张（从 `cards.rs` 挪来，施工 I-80 带上平台，I-81 中带上机器人号）：拼出来的文件名、交给页面的写法；号拼进文件名、
//! 规则的写法（施工 I-79）。
//!
//! 文件名：整个平台的卡，QQ 的照旧 `70-card-a.toml`、`70-card-b.toml`、`70-card-g-<群号>.toml`、`70-card-p-<号>.toml`，别的平台的
//! 在前面加 `<平台>~`（`70-card-tg~g-<群号>.toml`）；只管一个号的卡开头是 `71-card-`，带 `<平台>~<机器人号>~`
//! （`71-card-qq~3927564101~g-777.toml`），排在整个平台的后面，同一格照它的（设计 07 第 7.6 节）。`~` 不在号的写法里
//! （[`file_part`]），也不在平台名里，认得回来。

use serde_json::{Value, json};

use miyu_chat::{Venue, VenueKind};

use super::PREFIX;
use crate::chat::Id;

/// 卡片上没写平台的算它：I-80 以前存的卡片都是 OneBot 驱动的。也是文件名里不带平台的那一个。
pub const OLD_PLATFORM: &str = "qq";

/// 只管一个号的卡拼出来的文件名的开头：排在整个平台的卡（[`PREFIX`]）后面。
const BOT_PREFIX: &str = "71-card-";

/// 文件名里平台、机器人号和后面隔开的字。
const AFTER_PLATFORM: char = '~';

/// 一张卡管哪儿（施工 I-81 中）：一个平台；写了机器人号的只管那个号（一个连接），没写的管整个平台。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Scope {
    /// 平台：驱动报的名字。
    pub platform: String,
    /// 机器人号：没写的是整个平台。
    pub bot: Option<Id>,
}

impl Scope {
    /// 整个平台 `platform`。
    pub fn platform(platform: &str) -> Scope {
        Scope {
            platform: platform.to_string(),
            bot: None,
        }
    }

    /// 平台 `platform` 上的号 `bot`；`bot` 是空的就是整个平台。
    pub fn of(platform: &str, bot: Option<Id>) -> Scope {
        Scope {
            platform: platform.to_string(),
            bot,
        }
    }
}

/// 一张卡是哪一张：拼出来的文件名里写着。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Card {
    /// 会话默认值（施工 I-7 下二）：不分平台。
    Defaults,
    /// 「其他群」。
    OtherGroups(Scope),
    /// 「其他私聊」（施工 O-60）。
    OtherPrivates(Scope),
    /// 群号是它的那个群。
    Group(Scope, Id),
    /// 对方的号是它的那个私聊。
    Private(Scope, Id),
}

impl Card {
    /// 管哪儿：会话默认值不分平台，是空的。
    pub fn scope(&self) -> Option<&Scope> {
        match self {
            Card::Defaults => None,
            Card::OtherGroups(scope)
            | Card::OtherPrivates(scope)
            | Card::Group(scope, _)
            | Card::Private(scope, _) => Some(scope),
        }
    }

    /// 是哪个平台的：会话默认值不分平台，是空的。
    pub fn platform(&self) -> Option<&str> {
        self.scope().map(|scope| scope.platform.as_str())
    }

    /// 只管哪个号：管整个平台的、会话默认值是空的。
    pub fn bot(&self) -> Option<&Id> {
        self.scope().and_then(|scope| scope.bot.as_ref())
    }

    /// 拼出来的文件名（见模块开头）：照字排，同一个平台、同一个号的是「其他群」、「其他私聊」、群、私聊。
    pub(in crate::rules) fn file_name(&self) -> String {
        let (prefix, head) = match self.scope() {
            Some(Scope {
                platform,
                bot: Some(bot),
            }) => (
                BOT_PREFIX,
                format!(
                    "{platform}{AFTER_PLATFORM}{}{AFTER_PLATFORM}",
                    file_part(bot)
                ),
            ),
            Some(Scope { platform, .. }) if platform != OLD_PLATFORM => {
                (PREFIX, format!("{platform}{AFTER_PLATFORM}"))
            }
            _ => (PREFIX, String::new()),
        };
        let rest = match self {
            Card::Defaults => "0".to_string(),
            Card::OtherGroups(_) => "a".to_string(),
            Card::OtherPrivates(_) => "b".to_string(),
            Card::Group(_, id) => format!("g-{}", file_part(id)),
            Card::Private(_, id) => format!("p-{}", file_part(id)),
        };
        format!("{prefix}{head}{rest}.toml")
    }

    /// 照文件名认是哪一张卡：不是卡片拼的文件的是空的。
    pub fn parse(file: &str) -> Option<Card> {
        let name = file.strip_suffix(".toml")?;
        let (scope, rest) = if let Some(rest) = name.strip_prefix(BOT_PREFIX) {
            let (platform, rest) = rest.split_once(AFTER_PLATFORM)?;
            let (bot, rest) = rest.split_once(AFTER_PLATFORM)?;
            if !platform_ok(platform) {
                return None;
            }
            (Scope::of(platform, Some(from_file_part(bot)?)), rest)
        } else {
            let rest = name.strip_prefix(PREFIX)?;
            match rest.split_once(AFTER_PLATFORM) {
                Some((platform, rest)) if platform_ok(platform) && platform != OLD_PLATFORM => {
                    (Scope::platform(platform), rest)
                }
                Some(_) => return None,
                None if rest == "0" => return Some(Card::Defaults),
                None => (Scope::platform(OLD_PLATFORM), rest),
            }
        };
        match rest {
            "a" => return Some(Card::OtherGroups(scope)),
            "b" => return Some(Card::OtherPrivates(scope)),
            _ => {}
        }
        let (kind, id) = rest.split_once('-')?;
        let id = from_file_part(id)?;
        match kind {
            "g" => Some(Card::Group(scope, id)),
            "p" => Some(Card::Private(scope, id)),
            _ => None,
        }
    }

    /// 交给页面的写法：`{"kind": "defaults"}`、`{"kind": "other_groups", "platform": …}`、`{"kind": "group", "platform": …,
    /// "id": …}`……；只管一个号的多一格 `bot`。
    pub fn json(&self) -> Value {
        let mut shown = match self {
            Card::Defaults => return json!({"kind": "defaults"}),
            Card::OtherGroups(scope) => json!({"kind": "other_groups", "platform": scope.platform}),
            Card::OtherPrivates(scope) => {
                json!({"kind": "other_privates", "platform": scope.platform})
            }
            Card::Group(scope, id) => {
                json!({"kind": "group", "platform": scope.platform, "id": id})
            }
            Card::Private(scope, id) => {
                json!({"kind": "private", "platform": scope.platform, "id": id})
            }
        };
        if let Some(bot) = self.bot() {
            shown["bot"] = json!(bot);
        }
        shown
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
