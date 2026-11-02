//! 页面选中的连接（施工 I-85 下，设计 07 第二节第 3 条：首页的统计、日志跟着它）：参数 `connection` 写连接编号 `<平台>:<号>`，
//! 没写的是「全部」；会话浮窗「概况」的统计写 `venues`（施工 I-101）。I-81 以前的场所编号不带机器人号，同一个平台的几个连接分不开，照平台算。

use serde_json::Value;

use std::collections::BTreeSet;

use crate::counters::connection_of;

/// 只看哪些场所。
pub(super) enum Scope {
    /// 选中的那一个连接：平台、机器人的号。
    Connection { platform: String, bot: String },
    /// 一张会话卡管的几个场所（施工 I-101：浮窗「概况」的统计，`cards.overview` 交的 `venues`）。
    Venues(BTreeSet<String>),
}

impl Scope {
    /// 参数 `params` 里的 `connection` 或 `venues`（场所编号的数组）：都没写的是「全部」（空的）；两个都写的、`connection` 不是
    /// `<平台>:<号>` 的、`venues` 不是一串字的拒 `bad_params`。
    pub(super) fn of(params: &Value) -> Result<Option<Scope>, (String, String)> {
        let bad = || ("bad_params".to_string(), String::new());
        match (&params["connection"], &params["venues"]) {
            (Value::Null, Value::Null) => Ok(None),
            (Value::String(id), Value::Null) => {
                let (platform, bot) = id
                    .split_once(':')
                    .filter(|(platform, bot)| {
                        !platform.is_empty() && !bot.is_empty() && !bot.contains(':')
                    })
                    .ok_or_else(bad)?;
                Ok(Some(Scope::Connection {
                    platform: platform.to_string(),
                    bot: bot.to_string(),
                }))
            }
            (Value::Null, Value::Array(venues)) => venues
                .iter()
                .map(|venue| venue.as_str().map(str::to_string).ok_or_else(bad))
                .collect::<Result<_, _>>()
                .map(|venues| Some(Scope::Venues(venues))),
            _ => Err(bad()),
        }
    }

    /// 计数里按连接记的键 `key`（[`connection_of`]：`qq`、`qq:10001`）是不是这个连接的：平台对得上，带号的还得是这个号。只看几个
    /// 场所的不认按连接记的（按小时的没有按场所记）。
    pub(super) fn holds_key(&self, key: &str) -> bool {
        let Scope::Connection { platform, bot } = self else {
            return false;
        };
        match key.split_once(':') {
            Some((on, number)) => on == platform && number == bot,
            None => key == platform,
        }
    }
}

/// 场所 `venue` 在不在 `scope` 里：「全部」什么都在。
pub(super) fn within(scope: Option<&Scope>, venue: &str) -> bool {
    scope.is_none_or(|scope| match scope {
        Scope::Connection { .. } => scope.holds_key(connection_of(venue)),
        Scope::Venues(venues) => venues.contains(venue),
    })
}

#[cfg(test)]
mod tests;
