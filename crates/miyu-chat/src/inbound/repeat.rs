//! 复读（施工 I-28，`docs/blueprint/chat.md` 第二条「怎么走」第 3 条）：几个人接连发同一句话，真人只接一次。一串复读里只有第一条
//! 往下走，后面的只记下；终端管理员不算。是不是复读由外面照最近的人话算好交进来（[`Ctx::repeated`](super::Ctx::repeated)，用
//! [`repeats()`]），这一条规则只照它停。
//!
//! 「同一句话」照 [`echo()`] 比：去掉空白、标点、符号，字母一律小写，剩下的字算一个指纹。只有表情、图的没有指纹，不算复读。

use std::borrow::Borrow;

use miyu_kernel::id::ExternalId;
use miyu_kernel::time::Timestamp;

use super::{Clock, Ctx, Inbound, InboundRule, Outcome, Standing, Step, Why};

/// 复读这条规则。
pub(super) struct Rule;

impl InboundRule for Rule {
    fn judge(&self, msg: &Inbound, ctx: &Ctx, _clock: Clock) -> Step {
        match ctx.repeated && msg.said.standing != Standing::Admin {
            true => Step::Stop(Outcome::RecordOnly(Why::Repeated)),
            false => Step::Continue,
        }
    }
}

/// 一条人话里复读要看的：谁说的、什么时候、说了什么（[`echo()`] 的指纹）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spoken {
    /// 发的人。
    pub sender: ExternalId,
    /// 记下的时刻。
    pub at: Timestamp,
    /// 正文的指纹；没有字的是空的。
    pub echo: Option<u64>,
    /// 判过没有：进站链放行了的（正在判、判完了）是真的；睡着、限流这类只记下的是假的。只有判过的同一句才让后面的成了复读：
    /// 一串里谁都没被判过，后面的照常判。
    pub counted: bool,
}

/// 正文 `text` 的指纹：只留字母、数字和汉字这类字（`char::is_alphanumeric`），字母小写，照 UTF-8 算 FNV-1a。一个字都不剩的是空的。
pub fn echo(text: &str) -> Option<u64> {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    let mut hash = OFFSET;
    let mut any = false;
    for c in text.chars().filter(|c| c.is_alphanumeric()) {
        any = true;
        let mut buf = [0; 4];
        for lower in c.to_lowercase() {
            for byte in lower.encode_utf8(&mut buf).bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(PRIME);
            }
        }
    }
    any.then_some(hash)
}

/// 此刻（`now`）`sender` 说的、指纹是 `echo` 的这一条是不是在复读：`earlier` 是它之前的人话，照从新到旧。从此刻往回，一串同样
/// 指纹的话首尾相接、相邻两条隔得不超过 `window` 毫秒（含）；这一串里有别人说的、判过了的（[`Spoken::counted`]）就是复读。
/// 中间夹着别的话不断，隔得太久就断。同一个人自己重复的不算（那是补发，照顶替办）。没有指纹的不算。
pub fn repeats(
    echo: Option<u64>,
    sender: &ExternalId,
    now: Timestamp,
    earlier: impl IntoIterator<Item = impl Borrow<Spoken>>,
    window: i64,
) -> bool {
    let Some(echo) = echo else {
        return false;
    };
    let mut edge = now.unix_millis();
    for spoken in earlier {
        let spoken = spoken.borrow();
        if edge.saturating_sub(spoken.at.unix_millis()) > window {
            return false;
        }
        if spoken.echo == Some(echo) {
            if spoken.sender != *sender && spoken.counted {
                return true;
            }
            edge = spoken.at.unix_millis();
        }
    }
    false
}

/// `sender` 说的、指纹是 `echo` 的那一条后来有没有人跟着复读：`later` 是它之后的人话，照从旧到新；有别人说了同样指纹的就是。
/// 她回它时不再引用、@（`chat.md` 第五条第 6 条：回的是一串复读，不是哪一个人）。没有指纹的不算。
pub fn echoed(
    echo: Option<u64>,
    sender: &ExternalId,
    later: impl IntoIterator<Item = impl Borrow<Spoken>>,
) -> bool {
    let Some(echo) = echo else {
        return false;
    };
    later.into_iter().any(|spoken| {
        let spoken = spoken.borrow();
        spoken.echo == Some(echo) && spoken.sender != *sender
    })
}

#[cfg(test)]
mod tests;
