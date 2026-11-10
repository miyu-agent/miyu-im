//! 场所规则套到这一条消息上（`onebot.md` 第一条「群消息」第 3、4、6 条，施工 O-22）：套好的样子（`rules::Applied`）里这一步
//! 用到的几项。写法、怎么套由群聊内核管（`chat.md` 第一条），这里只照键取值。
//!
//! - `venue.session` 带的人格、预设、工作区（`persona`、`preset`、`workspace` → `cwd`），没设的不带。
//! - 发的人的身份：`managers` 里有的是 `manager`，别的 `member`；QQ 的群主、管理员不算（「施工时定的」第 58 条）。
//! - `show_ids`。
//! - 此刻睡没睡：照群聊内核的回合闸问（`gate()`），只交睡眠，闸说推迟就是睡着；时区照本机此刻的偏移（「施工时定的」
//!   第 65 条）。
//! - 进站链要的（施工 O-23，「群里怎么叫她」第 4、5 条）：限流、睡眠、能不能叫她、触发词，此刻（本机的钟和时区）。
//! - 线路规程（施工 O-23 下，第 10 条）：`discipline` 的写法，没设的是空的。
//! - 最多几条支线（施工 O-36，第 6 条）：`parallel`，没设的是 0。
//! - 对话模型（施工 O-58 补三）：`model`，找到会话以后经 `session.configure` 钉（`policy`，补四起人格、预设也照它换）。
//! - 群聊近况带多少条（施工 O-62）：`recent`，只给群的 `venue.session` 带，没设的不带。
//! - 使用限制（设计 08，施工 I-135）：卡片 `rules` 里的 `limit_5h`、`limit_week`、`limit_month`。

use std::time::{SystemTime, UNIX_EPOCH};

use miyu_chat::{Clock, Ctx, Gate, Moderation, Quota, Rate, Sleep, gate};
use miyu_client::ids::ExternalId;
use miyu_client::manifest::Value;
use miyu_client::time::{Timestamp, UtcOffset};

use crate::limits::{KEYS, Lines};
use crate::rules::Applied;

use super::Route;

impl Route {
    /// 这一刻套到场所 `venue` 上的场所规则（「场所规则和出厂数据」第 3、5 条）。
    pub(super) fn applied(&mut self, venue: &miyu_chat::Venue) -> Applied {
        self.rules.current(std::time::Instant::now()).at(venue)
    }
}

/// `venue.session` 的格和场所规则里对着的键（「群消息」第 3 条）。
const OPENING: [(&str, &str); 3] = [
    ("persona", "persona"),
    ("preset", "preset"),
    ("cwd", "workspace"),
];

/// `venue.session` 要带的几格：格名和值，照 [`OPENING`] 的先后，规则没设的不在里面。
pub(super) fn opening(applied: &Applied) -> Vec<(&'static str, &str)> {
    OPENING
        .iter()
        .filter_map(|(field, key)| text(applied, key).map(|value| (*field, value)))
        .collect()
}

/// 附加提示词（施工 O-58 上，`titled`）：规则的 `extra_prompt`，没设的是空的。
pub(super) fn extra_prompt(applied: &Applied) -> Option<&str> {
    text(applied, "extra_prompt")
}

/// 平台上是 `who` 的人在这个场所里的身份：规则的 `managers` 里有的是 `manager`，别的 `member`（「群消息」第 4 条）。
pub(super) fn role(applied: &Applied, who: &ExternalId) -> &'static str {
    let listed = match entry(applied, "managers") {
        Some(Value::List(managers)) => managers
            .iter()
            .any(|manager| matches!(manager, Value::Text(id) if id == who.as_str())),
        _ => false,
    };
    match listed {
        true => "manager",
        false => "member",
    }
}

/// 规则里键是 `key` 的真假：没写的、不是真假的是空的（施工 I-7 中三下一：忽略限流、忽略睡眠）。
pub(super) fn flag(applied: &Applied, key: &str) -> Option<bool> {
    match entry(applied, key) {
        Some(Value::Bool(on)) => Some(*on),
        _ => None,
    }
}

/// 规则的 `reaction`（施工 I-7 中三下二）：贴哪个表情，`off` 是不贴；没写的是空的（照桥的出厂）。
pub(super) fn reaction(applied: &Applied) -> Option<&str> {
    text(applied, "reaction")
}

/// 规则的 `show_ids` 是不是真的。
pub(super) fn show_ids(applied: &Applied) -> bool {
    matches!(entry(applied, "show_ids"), Some(Value::Bool(true)))
}

/// 此刻落在规则的睡觉时间里没有：没设、写成 `off` 的醒着。照群聊内核的回合闸问：只交睡眠（不限流、没禁言），闸说推迟就是
/// 睡着。本机的钟读不出（早于 1970 年、晚于 9999 年）的当醒着。
pub(super) fn asleep(applied: &Applied) -> bool {
    wakes(applied).is_some()
}

/// 此刻睡着的话什么时候醒（施工 I-89：看一眼顺延到睡觉时段结束）：醒着的、钟读不出的是空的。
pub(super) fn wakes(applied: &Applied) -> Option<Timestamp> {
    let (Some(sleep), Some(clock)) = (sleep(applied), clock()) else {
        return None;
    };
    let ctx = Ctx {
        rate: None,
        sleep: Some(sleep),
        allow: None,
        muted: false,
        repeated: false,
        turns: Vec::new(),
        moderation: Moderation {
            keywords: Vec::new(),
            base64: applied.params.base64,
        },
        quota: Quota::default(),
    };
    match gate(&ctx, clock) {
        Gate::Later(wake) => Some(wake),
        _ => None,
    }
}

/// 规则的限流（施工 O-23）：没设、`"0"`、写错的是空的（[`Rate::read`]）。
pub(super) fn rate(applied: &Applied) -> Option<Rate> {
    text(applied, "rate").and_then(Rate::read)
}

/// 规则的睡觉时间：没设、`off`、写错的是空的（[`Sleep::read`]）。
pub(super) fn sleep(applied: &Applied) -> Option<Sleep> {
    text(applied, "sleep").and_then(Sleep::read)
}

/// 规则的 `allow`（施工 O-23）：没设的是空的，进站链当能叫她。
pub(super) fn allow(applied: &Applied) -> Option<bool> {
    match entry(applied, "allow") {
        Some(Value::Bool(allow)) => Some(*allow),
        _ => None,
    }
}

/// 规则的触发词 `keywords`（施工 O-23，「群里怎么叫她」第 4 条）：没设的是空的。
pub(super) fn keywords(applied: &Applied) -> Vec<String> {
    match entry(applied, "keywords") {
        Some(Value::List(keywords)) => keywords
            .iter()
            .filter_map(|keyword| match keyword {
                Value::Text(keyword) => Some(keyword.to_string()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// 规则的线路规程 `discipline`（施工 O-23 下）：没设的是空的。
pub(super) fn discipline(applied: &Applied) -> Option<&str> {
    text(applied, "discipline")
}

/// 规则的 `parallel`（施工 O-36，「群里怎么叫她」第 6 条）：最多几条支线；没设的是 0（串行）。群聊内核查过是 0 到 16 的整数（施工 I-16 照旧版放宽）。
pub(super) fn parallel(applied: &Applied) -> u8 {
    match entry(applied, "parallel") {
        Some(Value::Int(count)) => u8::try_from(*count).unwrap_or(0),
        _ => 0,
    }
}

/// 对话模型（施工 O-58 补三）：规则的 `model`，没设的是空的。
pub(super) fn model(applied: &Applied) -> Option<&str> {
    text(applied, "model")
}

/// 群聊近况带多少条（施工 O-62）：规则的 `recent`，没设的是空的（核心只照字节预算）。
pub(super) fn recent(applied: &Applied) -> Option<i64> {
    match entry(applied, "recent") {
        Some(Value::Int(count)) => Some(*count),
        _ => None,
    }
}

/// 规则的开关 `glance`（施工 I-89）：时不时看一眼。没写的是不看。
pub(super) fn glances(applied: &Applied) -> bool {
    flag(applied, "glance") == Some(true)
}

/// 规则的 `glance_minutes`（施工 I-114）：她没定下一次时多久看一眼，分钟。没写的是空的（照桥的出厂）。
pub(super) fn glance_minutes(applied: &Applied) -> Option<u64> {
    match entry(applied, "glance_minutes") {
        Some(Value::Int(minutes)) => u64::try_from(*minutes).ok().filter(|minutes| *minutes > 0),
        _ => None,
    }
}

/// 规则的 `context`（施工 I-189）：会话的上下文上限，token 数。没写的是空的（照模型的窗口）。
pub(super) fn context(applied: &Applied) -> Option<u64> {
    match entry(applied, "context") {
        Some(Value::Int(tokens)) => u64::try_from(*tokens).ok().filter(|tokens| *tokens > 0),
        _ => None,
    }
}

/// 规则的 `glance_records`（施工 I-175）：看一眼那一轮最多交几条没回的新消息。没写的是空的（照桥的出厂）。
pub(super) fn glance_records(applied: &Applied) -> Option<usize> {
    match entry(applied, "glance_records") {
        Some(Value::Int(records)) => usize::try_from(*records)
            .ok()
            .filter(|records| *records > 0),
        _ => None,
    }
}

/// 看一眼时的额外提示词（施工 I-196）：规则的 `glance_note`，去掉首尾空白；没写的、空的是空的。
pub(super) fn glance_note(applied: &Applied) -> Option<&str> {
    text(applied, "glance_note")
        .map(str::trim)
        .filter(|note| !note.is_empty())
}

/// 会话的说话方式（施工 I-140）：规则的 `style_note`，去掉首尾空白；没写的、空的是空的。
pub(super) fn style_note(applied: &Applied) -> Option<&str> {
    text(applied, "style_note")
        .map(str::trim)
        .filter(|note| !note.is_empty())
}

/// 规则的使用限制（设计 08，施工 I-135）：卡片 `rules` 里的三条线，没写的、不是正整数的不限。
pub(super) fn limits(applied: &Applied) -> Lines {
    Lines::from_caps(KEYS.map(|key| match entry(applied, key) {
        Some(Value::Int(cap)) => u64::try_from(*cap).ok().filter(|cap| *cap > 0),
        _ => None,
    }))
}

/// 此刻：本机的钟，时区照本机此刻的偏移（「施工时定的」第 65 条）。钟读不出（早于 1970 年、晚于 9999 年）的是空的。
pub(super) fn clock() -> Option<Clock> {
    Some(Clock {
        now: now()?,
        offset: offset(),
    })
}

/// 规则设到的一项：没有规则设到的是空的。
fn entry<'a>(applied: &'a Applied, key: &str) -> Option<&'a Value> {
    applied.resolved.entries.get(key).map(|entry| &entry.value)
}

/// 规则设到的一项字 `key`（施工 O-58 补四：`policy` 照它取人格、预设）。
pub(super) fn text_of<'a>(applied: &'a Applied, key: &str) -> Option<&'a str> {
    text(applied, key)
}

/// 规则设到的一项字（名字、文字、睡眠的写法）。
fn text<'a>(applied: &'a Applied, key: &str) -> Option<&'a str> {
    match entry(applied, key) {
        Some(Value::Text(text)) => Some(text),
        _ => None,
    }
}

/// 此刻，照本机的钟。
pub(super) fn now() -> Option<Timestamp> {
    let since = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    Timestamp::from_unix_millis(i64::try_from(since.as_millis()).ok()?)
}

/// 本机此刻的时区偏移，到分钟（照核心给会话钉下时区的算法，`miyu-endpoint` 的 `sessions.rs`）。读出来超了范围的当 UTC。
fn offset() -> UtcOffset {
    let minutes = jiff::Zoned::now().offset().seconds() / 60;
    UtcOffset::from_minutes(minutes).unwrap_or(UtcOffset::UTC)
}

#[cfg(test)]
pub(super) mod tests;
