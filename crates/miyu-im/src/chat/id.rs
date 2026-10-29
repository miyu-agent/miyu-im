//! 平台上的号（施工 I-79，设计 07 第 7.2 节）：群、人、机器人的号一律当字。QQ 的号是整数，QQ 官方机器人、微信的是一串字，
//! Telegram 的群号是负数；宿主不解析、不比大小，只有驱动在边上照自己的平台换。
//!
//! 整数写法的号写成 JSON 时照旧是数（[`Serialize`]）：只用 QQ 的人存下来的文件、给页面和她的字一个字节不变。

use std::cmp::Ordering;
use std::fmt;
use std::sync::Arc;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 号最长几个字节：场所编号要放下平台、机器人号、种类和号，核心限编号 128 字节（QQ 官方机器人的 openid 32 个字，微信的 20 来个）。
pub const MAX_LEN: usize = 48;

/// JSON 的数照双精度读，超过它的整数写成字（浏览器读得准）。
const EXACT: i64 = (1 << 53) - 1;

/// 平台上的一个号：非空、最长 48 字节（`MAX_LEN`）、不含 `:`、空白和控制字符（群聊内核拼场所编号照同样的规矩查）。
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Id(Arc<str>);

impl Id {
    /// 照字 `text` 造；空的、太长的、带 `:`、空白和控制字符的是空的（驱动照自己的平台转义）。
    pub fn new(text: &str) -> Option<Id> {
        let fits = !text.is_empty()
            && text.len() <= MAX_LEN
            && !text
                .chars()
                .any(|char| char == ':' || char.is_whitespace() || char.is_control());
        fits.then(|| Id(Arc::from(text)))
    }

    /// 号的字。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 号是十进制整数的（QQ 的号、Telegram 的群号）：照整数读；别的写法（前面带 `+`、`0`）、读不成整数的是空的。
    pub fn number(&self) -> Option<i64> {
        let number: i64 = self.0.parse().ok()?;
        (number.to_string() == *self.0).then_some(number)
    }
}

impl From<i64> for Id {
    fn from(number: i64) -> Id {
        Id(Arc::from(number.to_string()))
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 照原样印（同整数的号以前印的样子），日志里不带引号。
impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 整数写法的照数比，排在别的前面；别的照字比。只有 QQ 的时候先后和以前一样。
impl Ord for Id {
    fn cmp(&self, other: &Id) -> Ordering {
        match (self.number(), other.number()) {
            (Some(left), Some(right)) => left.cmp(&right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => self.0.cmp(&other.0),
        }
    }
}

impl PartialOrd for Id {
    fn partial_cmp(&self, other: &Id) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Serialize for Id {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.number() {
            Some(number) if (-EXACT..=EXACT).contains(&number) => serializer.serialize_i64(number),
            _ => serializer.serialize_str(&self.0),
        }
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Id, D::Error> {
        deserializer.deserialize_any(Seen)
    }
}

/// 读的时候数、字都认。
struct Seen;

impl Visitor<'_> for Seen {
    type Value = Id;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an id: an integer, or text without ':', spaces and control characters")
    }

    fn visit_i64<E: de::Error>(self, number: i64) -> Result<Id, E> {
        Ok(Id::from(number))
    }

    fn visit_u64<E: de::Error>(self, number: u64) -> Result<Id, E> {
        i64::try_from(number)
            .map(Id::from)
            .map_err(|_| E::invalid_value(de::Unexpected::Unsigned(number), &self))
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Id, E> {
        Id::new(text).ok_or_else(|| E::invalid_value(de::Unexpected::Str(text), &self))
    }
}

#[cfg(test)]
mod tests;
