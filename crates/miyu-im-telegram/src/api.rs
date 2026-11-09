//! Bot API 的一次调用（设计 07 第 3.5 节，照 Bot API 10.3 的文档）：`POST <地址>/bot<令牌>/<方法>`，参数是 JSON，回的是
//! `{"ok": true, "result": …}` 或 `{"ok": false, "error_code", "description", "parameters": {"retry_after"}}`。
//!
//! - 令牌在网址里：出错时 reqwest 的错带着网址，一律先 `without_url` 再记、再交出去，运行日志、状态里都不出现令牌。
//! - 代理一个连接一份（[`Proxy`]）：跟随联网设置（照环境变量 `HTTPS_PROXY` 这些，同核心）、直连、照写的地址（`http`、`https`、
//!   `socks5`）。
//! - 几种没成分开交（[`Failure`]）：401 令牌不对、409 别处也在收（设计 07：文档没写、实际如此）、429 照 `retry_after` 等，
//!   连不上、等不到、回的不是 JSON 的当网络出错，别的照 Telegram 说的原话。

use std::time::Duration;

use miyu_client::settings::Secret;
use serde_json::Value;

/// 连上 Telegram 最多等多久。
const CONNECT: Duration = Duration::from_secs(15);

/// 一个连接的代理（设计 07 第 3.5 节：每个连接能单独设）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proxy {
    /// 跟随 Miyu 的联网设置：照环境变量（`HTTPS_PROXY`、`ALL_PROXY`、`NO_PROXY` 这些，同核心发请求）。
    Environment,
    /// 直连。
    Direct,
    /// 照这个地址（`http://`、`https://`、`socks5://`）。
    Address(String),
}

impl Proxy {
    /// 配置里存的写法：`env` 是跟随联网设置（空的也认：核心的一列字不收空字，存的是 `env`），`direct` 是直连，别的是地址（去掉首尾空白）。
    pub fn parse(written: &str) -> Proxy {
        match written.trim() {
            "" | "env" => Proxy::Environment,
            "direct" => Proxy::Direct,
            address => Proxy::Address(address.to_string()),
        }
    }

    /// 存回配置的写法（[`Proxy::parse`] 反过来）。
    pub fn written(&self) -> &str {
        match self {
            Proxy::Environment => "env",
            Proxy::Direct => "direct",
            Proxy::Address(address) => address,
        }
    }
}

/// 一次调用没成。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// 401：令牌不对、被吊销了。
    Unauthorized,
    /// 409：这个令牌别处也在收（另一个长轮询，或者设了 webhook）。
    Conflict,
    /// 429：照它说的等这么久再来。
    Retry(Duration),
    /// 连不上、等不到、回的读不懂：原因（不带网址）。
    Network(String),
    /// Telegram 回了没成：错误码、它说的原话。
    Failed {
        /// `error_code`。
        code: i64,
        /// `description`。
        description: String,
    },
}

/// 一个机器人令牌的调用口。`Debug` 不印令牌。
#[derive(Clone)]
pub struct Api {
    client: reqwest::Client,
    /// `<地址>/bot<令牌>`：调用照它拼。
    base: String,
}

impl std::fmt::Debug for Api {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Api(…)")
    }
}

impl Api {
    /// 照 Bot API 的地址 `api`（出厂 `https://api.telegram.org`，测试指本机的假服务器）、令牌、代理造一个。
    ///
    /// # Errors
    ///
    /// 代理的地址读不懂、客户端造不出来：原因。
    pub fn new(api: &str, token: &Secret, proxy: &Proxy) -> Result<Api, String> {
        let mut builder = reqwest::Client::builder()
            // 整个进程共用一份 TLS 配置（同核心的客户端，施工 V-5 上）：证书只读一次。
            .use_preconfigured_tls(miyu_client::http::tls())
            .user_agent(concat!("miyu-im/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(CONNECT);
        builder = match proxy {
            // reqwest 自己照环境变量走代理（`HTTPS_PROXY`、`ALL_PROXY`、`NO_PROXY`，大小写都认）。
            Proxy::Environment => builder,
            Proxy::Direct => builder.no_proxy(),
            Proxy::Address(address) => builder.proxy(
                reqwest::Proxy::all(address.as_str())
                    .map_err(|error| format!("bad proxy: {}", error.without_url()))?,
            ),
        };
        let client = builder
            .build()
            .map_err(|error| format!("no client: {}", error.without_url()))?;
        Ok(Api {
            client,
            base: format!("{}/bot{}", api.trim_end_matches('/'), token.expose()),
        })
    }

    /// 调 `method`，参数 `params`，最多等 `wait`：交回 `result`。
    ///
    /// # Errors
    ///
    /// 见 [`Failure`]。
    pub async fn call(
        &self,
        method: &str,
        params: &Value,
        wait: Duration,
    ) -> Result<Value, Failure> {
        let body =
            serde_json::to_vec(params).map_err(|error| Failure::Network(error.to_string()))?;
        let response = self
            .client
            .post(format!("{}/{method}", self.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .timeout(wait)
            .send()
            .await
            .map_err(network)?;
        let status = response.status().as_u16();
        let bytes = response.bytes().await.map_err(network)?;
        read(status, &bytes)
    }
}

/// reqwest 的错：去掉网址（带着令牌）再说。
fn network(error: reqwest::Error) -> Failure {
    let error = error.without_url();
    let kind = if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connect"
    } else {
        "request"
    };
    Failure::Network(format!("{kind}: {error}"))
}

/// 读回应：HTTP 状态 `status`、正文 `bytes`。
fn read(status: u16, bytes: &[u8]) -> Result<Value, Failure> {
    let Ok(reply) = serde_json::from_slice::<Value>(bytes) else {
        return Err(match status {
            401 => Failure::Unauthorized,
            409 => Failure::Conflict,
            _ => Failure::Network(format!("http {status}, not json")),
        });
    };
    if reply["ok"] == true {
        return Ok(reply["result"].clone());
    }
    let code = reply["error_code"].as_i64().unwrap_or(i64::from(status));
    match code {
        401 => Err(Failure::Unauthorized),
        409 => Err(Failure::Conflict),
        429 => {
            let seconds = reply["parameters"]["retry_after"].as_u64().unwrap_or(1);
            Err(Failure::Retry(Duration::from_secs(seconds.max(1))))
        }
        _ => Err(Failure::Failed {
            code,
            description: reply["description"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .to_string(),
        }),
    }
}

#[cfg(test)]
mod tests;
