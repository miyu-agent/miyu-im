//! 发言排行（施工 I-20，照旧版 `get_real_chat_activity_ranking`；2026-10-21 项目主人定要做）：`qq_activity_ranking {days?, limit?}`，
//! 只给群。几天、列几个认好，交给 im 照它自己按人数的条数排（`Effect::Ranking`）；名字照群成员的缓存，没有的写号。

use serde_json::Value;

use miyu_im::chat::To;
use miyu_im::toolset::{Call, Effect, Planned};

use super::args::{bad_args, group_only, qq_number};

/// 没写 `days` 的照几天。
const DAYS: u32 = 7;

/// 没写 `limit` 的列几个。
const LIMIT: usize = 10;

/// 最多列几个。
const MOST: usize = 50;

/// 读参数（纯逻辑）：`days` 是 1、7、30（没写的 7），`limit` 是 1 到 [`MOST`]（没写的 10）。写错的交回哪一个不对。
pub(crate) fn wanted(args: &Value) -> Result<(u32, usize), &'static str> {
    let days = match &args["days"] {
        Value::Null => DAYS,
        value => match qq_number(value) {
            Some(1) => 1,
            Some(7) => 7,
            Some(30) => 30,
            _ => return Err("days must be 1, 7 or 30"),
        },
    };
    let limit = match &args["limit"] {
        Value::Null => LIMIT,
        value => match qq_number(value).and_then(|limit| usize::try_from(limit).ok()) {
            Some(limit) if limit <= MOST => limit,
            _ => return Err("limit must be 1 to 50"),
        },
    };
    Ok((days, limit))
}

/// `qq_activity_ranking` 的一次调用（模块的说明）。
pub(super) fn ranking(call: &Call) -> Planned {
    if !matches!(call.to, To::Group(_)) {
        return group_only(call);
    }
    match wanted(&call.args) {
        Ok((days, limit)) => Planned::Host(Effect::Ranking { days, limit }),
        Err(detail) => bad_args(call, detail),
    }
}

#[cfg(test)]
mod tests;
