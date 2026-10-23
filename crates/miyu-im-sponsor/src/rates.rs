//! 查汇率（设计 03 第一节第 4 条）：open.er-api.com（免费、不用 key，旧版同一家）。记账那一刻查一次，查到的冻结进那一笔；
//! 查不到的交回空的，那一笔记原币种、标「未换算」，后台补，不猜（「算不准的钱不显示」）。
//!
//! 地址照环境变量 `MIYU_SPONSOR_RATES_URL` 改（测试连本机的假服务），后面接原币种：`<地址><币种>`，回
//! `{"result": "success", "rates": {"CNY": 7.1, …}}`。

use std::time::Duration;

use miyu_client::http::{Get, Got, Proxy};
use serde_json::Value;

/// 出厂的地址。
const URL: &str = "https://open.er-api.com/v6/latest/";

/// 一次最多等多久。
const TIMEOUT: Duration = Duration::from_secs(10);

/// 回的最多多少字节。
const LIMIT: usize = 256 * 1024;

/// 查汇率的一头：测试换成假的。
pub trait Rates: Send + Sync {
    /// 1 `from` 合多少 `to`；查不到的是空的。
    fn rate(&self, from: &str, to: &str) -> Option<f64>;
}

/// 上网查。
pub struct Web {
    url: String,
}

impl Web {
    /// 照环境变量、出厂的地址。
    pub fn new() -> Web {
        let url = std::env::var("MIYU_SPONSOR_RATES_URL").unwrap_or_else(|_| URL.to_string());
        Web { url }
    }
}

impl Rates for Web {
    fn rate(&self, from: &str, to: &str) -> Option<f64> {
        let url = format!("{}{from}", self.url);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()?;
        let bytes = runtime.block_on(download(&url))?;
        parsed(&bytes, to)
    }
}

/// 下一次：回环的不走代理，别的照环境变量（同 im 取图）。
async fn download(url: &str) -> Option<Vec<u8>> {
    let proxy = if miyu_client::http::is_loopback_url(url) {
        Proxy::Off
    } else {
        Proxy::FromEnvironment
    };
    let client = miyu_client::http::fetcher(proxy).ok()?;
    let got = miyu_client::http::get(Get {
        client: &client,
        url,
        headers: &[],
        etag: None,
        timeout: TIMEOUT,
        limit: LIMIT,
    })
    .await;
    match got {
        Ok(Got::Body { bytes, .. }) => Some(bytes),
        Ok(Got::NotModified) => None,
        Err(error) => {
            eprintln!("rate not fetched: {error}");
            None
        }
    }
}

/// 回的那一份里 1 原币种合多少 `to`（纯逻辑）：`result` 是 `success`、那一项是正数才算。
pub fn parsed(bytes: &[u8], to: &str) -> Option<f64> {
    let body: Value = serde_json::from_slice(bytes).ok()?;
    if body["result"] != "success" {
        return None;
    }
    body["rates"][to]
        .as_f64()
        .filter(|rate| rate.is_finite() && *rate > 0.0)
}

#[cfg(test)]
mod tests;
