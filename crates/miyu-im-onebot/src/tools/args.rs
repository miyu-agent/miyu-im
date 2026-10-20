//! 参数的读法（施工 I-17：照旧版，参数写错的说哪个参数不对、期望什么，不再答「没有这件工具」）：纯逻辑。号照正整数读，写成数字的字
//! （`"12345"`）也认；号的列表照数组读，写成数组的 JSON 字的也认（模型偶尔把数组写成字）。写错的交回一句英文短句（`bad-args`
//! 的 `detail`），说哪个参数、期望什么。

use serde_json::Value;

use miyu_im::toolset::{Call, Planned};

/// 写错了的那一句：哪个参数、期望什么（`bad-args` 的 `detail`）。
pub(crate) type Bad = &'static str;

/// 一个号 `value`（纯逻辑）：正整数，或者写成正整数的字（去掉首尾空白）。别的是空的。
pub(crate) fn qq_number(value: &Value) -> Option<i64> {
    let number = match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    };
    number.filter(|number| *number > 0)
}

/// 参数里的一个号 `args[key]`：没给的是 `Ok(None)`，写错的交回 `bad`。
pub(crate) fn optional_number(args: &Value, key: &str, bad: Bad) -> Result<Option<i64>, Bad> {
    match &args[key] {
        Value::Null => Ok(None),
        value => qq_number(value).map(Some).ok_or(bad),
    }
}

/// 号的列表 `value`（纯逻辑）：数组（写成数组的 JSON 字的也认），每一个照 [`qq_number`]，去重、照先后，1 到 `most` 个。有写错
/// 的、空的、多了的是空的。
pub(crate) fn qq_numbers(value: &Value, most: usize) -> Option<Vec<i64>> {
    let parsed;
    let list = match value {
        Value::Array(list) => list,
        Value::String(text) => {
            parsed = serde_json::from_str::<Value>(text.trim()).ok()?;
            parsed.as_array()?
        }
        _ => return None,
    };
    let mut found: Vec<i64> = Vec::new();
    for one in list {
        let number = qq_number(one)?;
        if !found.contains(&number) {
            found.push(number);
        }
    }
    (1..=most).contains(&found.len()).then_some(found)
}

/// 参数里的一段字 `args[key]`：没给的是 `Ok(None)`，不是字的交回 `bad`。
pub(crate) fn optional_text<'a>(
    args: &'a Value,
    key: &str,
    bad: Bad,
) -> Result<Option<&'a str>, Bad> {
    match &args[key] {
        Value::Null => Ok(None),
        Value::String(text) => Ok(Some(text)),
        _ => Err(bad),
    }
}

/// 参数写错了答的那一句。
pub(crate) fn bad_args(call: &Call, detail: &str) -> Planned {
    Planned::Now(call.answers.say("bad-args", &[("detail", detail)], true))
}

/// 只给群的工具在私聊里调到了答的那一句（`venues` 不给，照说不会；登记缓存是旧的时会）。
pub(crate) fn group_only(call: &Call) -> Planned {
    Planned::Now(call.answers.say("group-only", &[], true))
}

#[cfg(test)]
mod tests;
