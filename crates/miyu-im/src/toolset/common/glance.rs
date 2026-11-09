//! 定下一次什么时候看一眼（设计 05 第二节第 4 条，施工 I-89 下；施工 I-183 下下从 OneBot 驱动挪来，`qq_glance`、`tg_glance` 同用）：
//! `<平台>_glance {minutes}`，私聊、群都给，只在开着时不时看一眼的场所里
//! 有（[`crate::toolset::GLANCE_PLUGIN`]）。几分钟认好，交给 im 换定时（`Effect::Glance`）；小于 1 的当 1（防出 bug 时一分钟叫醒好几次），
//! 不设上限。

use serde_json::Value;

use crate::toolset::{Call, Effect, Planned};

use super::args::bad_args;

/// 读参数（纯逻辑）：`minutes` 是整数，小于 1 的当 1；没写、不是整数的交回哪里不对。
///
/// # Errors
///
/// 没写、不是整数的交回哪里不对。
pub fn minutes(args: &Value) -> Result<u64, &'static str> {
    let whole = match &args["minutes"] {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    };
    match whole {
        Some(minutes) => Ok(u64::try_from(minutes).unwrap_or(0).max(1)),
        None => Err("minutes must be a whole number"),
    }
}

/// 一次调用（模块的说明）。
pub fn glance(call: &Call) -> Planned {
    match minutes(&call.args) {
        Ok(minutes) => Planned::Host(Effect::Glance { minutes }),
        Err(detail) => bad_args(call, detail),
    }
}

#[cfg(test)]
mod tests;
