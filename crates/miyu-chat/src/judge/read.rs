//! 读判官的回答（`chat.md` 第六条「怎么走」第 4 条）。
//!
//! 各家模型回的样子不一样：有的包在 ` ```json ` 里，有的前后带几句话。读的时候宽（施工 I-16 照旧版 judge.rs）：只认第一个 `{`
//! 到最后一个 `}` 那一段；五维少了、不是数的按 0 算（写成字的数照读）；`should_reply`、`to_bot` 认 `true`、`"是"`、`1` 这类写法，
//! 读不出的当没说；`severity` 读不出的当没查、`reason` 当空。
//! 判不了的照不回算，由外面记一笔 `ext.im.chat.decided`（「怎么走」第 5 条）。

use serde_json::{Map, Value};

use crate::Judgement;

use super::Mode;

/// 五维在回答里的名字，照相关、意愿、社交、时机、连贯的先后，和 [`Judgement::scores`] 一一对上。是 `answer.txt`
/// 里定的格式的一部分，那份原文改了这里跟着改。
const DIMENSIONS: [&str; 5] = ["relevance", "willingness", "social", "timing", "continuity"];

/// 分和违规的严重程度的上限：0 到 10（`answer.txt`、`reply.txt`、`violations.txt`）。
const MAX: f64 = 10.0;

/// 判官的回答读不出来：当判不了，照不回算，记进 `ext.im.chat.decided` 时写明是哪一种（「怎么走」第 5 条）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unreadable {
    /// 回答里找不到第一个 `{` 到最后一个 `}` 那一段，或者那一段读不成 JSON 对象：空字、只有话、半截的 JSON、数组。
    NoObject,
    /// 五维少了这一维，或者它不是数（`null`、写成字的数也算不是）。带着这一维在回答里的名字。
    Dimension(&'static str),
    /// 只查违规的那一次没有 `severity`，或者它不是数：这一次问的就是它。
    NoSeverity,
}

/// 读判官的回答：从第一个 `{` 到最后一个 `}` 照 JSON 读成一个对象，读出 [`Judgement`]。
///
/// - 五维少了、不是数的按 0；小于 0 的当 0，大于 10 的当 10。
/// - `should_reply`、`to_bot` 照 `flexible` 读，读不出的是没说（`should_reply` 不加不减，`to_bot` 当不是）。
/// - `severity` 是数的夹到 0 到 10、四舍五入成整数；不是数（包括没有）的当没查（`None`）。
/// - `reason` 不是字（包括没有）的当空；超过 `reason_chars` 个字符的截到 `reason_chars`（出厂 500：只进日志，太长的截掉，免得一条日志被撑大，
///   「怎么走」第 4 条；数由外面交进来，从第八条的 `Params::judge` 拿，这里不写死）。
///
/// `mode` 是这一次问的是什么：[`Mode::ModerationOnly`] 问的就是违规，没有 `severity` 判不了。
///
/// # Errors
///
/// 找不到对象、读不成对象（[`Unreadable::NoObject`]）；只查违规的那一次没有 `severity`（[`Unreadable::NoSeverity`]）。
pub fn read(answer: &str, mode: Mode, reason_chars: usize) -> Result<Judgement, Unreadable> {
    let object = object(answer).ok_or(Unreadable::NoObject)?;
    let mut scores = [0.0; 5];
    for (score, name) in scores.iter_mut().zip(DIMENSIONS) {
        let value = object.get(name).and_then(number).unwrap_or_default();
        *score = value.clamp(0.0, MAX);
    }
    let flag = |name: &str| object.get(name).and_then(flexible);
    let severity = object
        .get("severity")
        .and_then(Value::as_f64)
        // 夹到 0 到 10 以后四舍五入，落在 u8 里，`as` 不会截。
        .map(|severity| severity.clamp(0.0, MAX).round() as u8);
    if mode == Mode::ModerationOnly && severity.is_none() {
        return Err(Unreadable::NoSeverity);
    }
    let reason = object
        .get("reason")
        .and_then(Value::as_str)
        .map(|reason| reason.chars().take(reason_chars).collect())
        .unwrap_or_default();
    Ok(Judgement {
        scores,
        should_reply: flag("should_reply"),
        to_bot: flag("to_bot") == Some(true),
        severity,
        reason,
    })
}

/// 一个数：是数的照读，写成字的数也读（`"7"`）。
fn number(value: &Value) -> Option<f64> {
    match value {
        Value::String(text) => text.trim().parse().ok(),
        _ => value.as_f64(),
    }
}

/// 宽着读一个是非（照旧版 `flexible_bool`）：布尔；数（0 是否）；`true`、`yes`、`1`、`reply`、`是`、`回复`、`应该回复` 是，
/// `false`、`no`、`0`、`no_reply`、`否`、`不回复` 否；别的读不出。
fn flexible(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::Number(value) => value.as_f64().map(|value| value != 0.0),
        Value::String(value) => match value.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" | "reply" | "应该回复" | "回复" | "是" => Some(true),
            "false" | "no" | "0" | "no_reply" | "不回复" | "否" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// 第一个 `{` 到最后一个 `}`（含两头）读成的 JSON 对象；找不到、读不成对象的是 `None`。取最外的一段，理由里带着花括号
/// 的、包在代码块里的、前后有话的都认得。
fn object(answer: &str) -> Option<Map<String, Value>> {
    let start = answer.find('{')?;
    let end = answer.rfind('}')?;
    let slice = answer.get(start..=end)?;
    match serde_json::from_str(slice).ok()? {
        Value::Object(object) => Some(object),
        _ => None,
    }
}
