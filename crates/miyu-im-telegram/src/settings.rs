//! Telegram 驱动自己的配置项（施工 I-183 中上，设计 07 第 3.5 节、第 7.7 节）：宿主照 [`NAMES`] 挑出 `im.tg.` 那一组交过来。
//!
//! - 连接表：两个藏起来的列表 `tokens`（密钥的引用，核心交配置时换成真值）、`proxies`（字：`env` 跟随联网设置（空的也认）、`direct` 直连、别的
//!   是代理地址），第几项是第几个连接；照 `tokens` 算几个连接，`proxies` 少了的照跟随联网设置，多的不认。令牌是空的、不是字的那一项
//!   照旧算一个连接（报「没有令牌」），不连。同一个令牌写了两次的只认第一个（两处收同一个令牌 Telegram 回 409）。
//! - `api`：Bot API 的地址，藏起来，出厂 `https://api.telegram.org`；测试指本机的假服务器，自建 Bot API 服务的人也能改。

use serde_json::{Map, Value};

use miyu_client::settings::Secret;

use crate::api::Proxy;

/// 连接表的令牌那一列。
const TOKENS: &str = "tokens";

/// 连接表的代理那一列。
const PROXIES: &str = "proxies";

/// Bot API 的地址。
const API: &str = "api";

/// 出厂的 Bot API 地址：清单里没写默认值、读不出的照它。
pub const OFFICIAL: &str = "https://api.telegram.org";

/// 驱动的配置项：清单里的名字（`Driver::settings`）。
pub const NAMES: &[&str] = &[TOKENS, PROXIES, API];

/// 其中是密钥的（`Driver::secrets`）。
pub const SECRETS: &[&str] = &[TOKENS];

/// 一个连接。`Debug` 只印 `Secret(…)`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    /// 第几个（从 0 数）：页面照它认是连接表里的哪一项。
    pub index: usize,
    /// 机器人的令牌；没写、取不到的是空的。
    pub token: Option<Secret>,
    /// 代理。
    pub proxy: Proxy,
}

/// 读宿主交来的 `config`：连接表和 Bot API 的地址（`default` 照名字给清单的默认值）。
pub fn read(
    config: &Map<String, Value>,
    default: impl Fn(&str) -> Option<Value>,
) -> (Vec<Connection>, String) {
    let api = config
        .get(API)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|api| api.starts_with("http://") || api.starts_with("https://"))
        .map(str::to_string)
        .or_else(|| default(API).and_then(|api| api.as_str().map(str::to_string)))
        .unwrap_or_else(|| OFFICIAL.to_string());
    let column = |name| {
        config
            .get(name)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let (tokens, proxies) = (column(TOKENS), column(PROXIES));
    let mut all: Vec<Connection> = Vec::new();
    for (index, written) in tokens.iter().enumerate() {
        let token = written
            .as_str()
            .and_then(|token| Secret::new(token.trim()).ok());
        if token.is_some() && all.iter().any(|one| one.token == token) {
            continue;
        }
        let proxy = proxies
            .get(index)
            .and_then(Value::as_str)
            .map_or(Proxy::Environment, Proxy::parse);
        all.push(Connection {
            index,
            token,
            proxy,
        });
    }
    (all, api)
}

#[cfg(test)]
mod tests;
