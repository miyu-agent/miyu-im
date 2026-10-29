//! 页面选中的连接（施工 I-85 下，设计 07 第二节第 3 条：首页的统计、日志跟着它）：参数 `connection` 写连接编号 `<平台>:<号>`，
//! 没写的是「全部」。I-81 以前的场所编号不带机器人号，同一个平台的几个连接分不开，照平台算。

use serde_json::Value;

use crate::counters::connection_of;

/// 选中的那一个连接。
pub(super) struct Scope {
    /// 平台。
    platform: String,
    /// 机器人的号。
    bot: String,
}

impl Scope {
    /// 参数 `params` 里的 `connection`：没写的是「全部」（空的）；不是字的、不是 `<平台>:<号>` 的拒 `bad_params`。
    pub(super) fn of(params: &Value) -> Result<Option<Scope>, (String, String)> {
        let bad = || ("bad_params".to_string(), String::new());
        let id = match &params["connection"] {
            Value::Null => return Ok(None),
            Value::String(id) => id,
            _ => return Err(bad()),
        };
        let (platform, bot) = id
            .split_once(':')
            .filter(|(platform, bot)| !platform.is_empty() && !bot.is_empty() && !bot.contains(':'))
            .ok_or_else(bad)?;
        Ok(Some(Scope {
            platform: platform.to_string(),
            bot: bot.to_string(),
        }))
    }

    /// 计数里按连接记的键 `key`（[`connection_of`]：`qq`、`qq:10001`）是不是这个连接的：平台对得上，带号的还得是这个号。
    pub(super) fn holds_key(&self, key: &str) -> bool {
        match key.split_once(':') {
            Some((platform, bot)) => platform == self.platform && bot == self.bot,
            None => key == self.platform,
        }
    }
}

/// 场所 `venue` 在不在 `scope` 里：「全部」什么都在。
pub(super) fn within(scope: Option<&Scope>, venue: &str) -> bool {
    scope.is_none_or(|scope| scope.holds_key(connection_of(venue)))
}

#[cfg(test)]
mod tests;
