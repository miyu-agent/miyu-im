//! 不是本机回环的地址（QQ 的图床、合并转发里段的 `url`、群文件的地址）怎么下（施工 I-96，发布前第二轮审查 I2-3）：这些地址是
//! 平台给的，以后接别的实现、别的平台就是别人能写的；照核心的地址闸（门面 `miyu_client::http` 转出的，`net.md`「怎么走」第 3 条）挡住内网、
//! 本机。
//!
//! - 每一跳先过闸；解析出来的地址都是公网的才去，不走代理的把解析好的地址钉进这一跳（查过的就是连上的，不给第二次解析换地址）。
//!   设了代理、这个地址该走代理的照走（闸只看样子，解析交给代理）。
//! - 跳转不交给客户端：自己跟，最多 [`HOPS`] 跳，每一跳重新过闸。
//! - 读到 `limit` 字节就停，超了的算错（同原来的下载）。
//!
//! NapCat 回的本机地址（回环）不走这里：照旧直接下（`super::download`）。

use std::time::Duration;

use miyu_client::http::{Guard, Proxies, Resolved};
use reqwest::redirect::Policy;
use reqwest::{Client, Url};

/// 最多跟几跳跳转。
pub(super) const HOPS: usize = 3;

/// 照 `guard`、`proxies` 下 `url`，最多 `limit` 字节，整个最多 `timeout`。
pub(super) async fn download(
    url: &str,
    (guard, proxies): (&Guard, &Proxies),
    timeout: Duration,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let mut url = Url::parse(url).map_err(|error| format!("url: {error}"))?;
    for _ in 0..=HOPS {
        let client = client(&url, (guard, proxies), timeout).await?;
        let response = client
            .get(url.clone())
            .timeout(timeout)
            .send()
            .await
            .map_err(|error| format!("url: {error}"))?;
        let status = response.status();
        if status.is_redirection() {
            let next = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| format!("url: {status} without a location"))?;
            url = url.join(next).map_err(|error| format!("url: {error}"))?;
            continue;
        }
        if !status.is_success() {
            return Err(format!("url: {status}"));
        }
        return body(response, limit).await;
    }
    Err("url: too many redirects".to_string())
}

/// 这一跳的客户端：过闸，照解析的结果钉地址或者走代理；跳转一个都不跟。
async fn client(
    url: &Url,
    (guard, proxies): (&Guard, &Proxies),
    timeout: Duration,
) -> Result<Client, String> {
    let refused = || {
        format!(
            "url: {} is not a public address",
            url.host_str().unwrap_or_default()
        )
    };
    let builder = Client::builder()
        .use_preconfigured_tls(miyu_client::http::tls())
        .redirect(Policy::none())
        .connect_timeout(timeout);
    let builder = match proxies.via(url) {
        // 走代理的：闸只看样子（代理替我们解析，内网的代理那边挡）。
        Some(via) => {
            if !guard.shape(url) {
                return Err(refused());
            }
            builder.proxy(via.proxy().map_err(|error| format!("url: {error}"))?)
        }
        None => match guard.pass(url, timeout).await.map_err(|_| refused())? {
            Resolved::Literal => builder.no_proxy(),
            Resolved::Addresses(addresses) => {
                let host = url.host_str().unwrap_or_default();
                builder.no_proxy().resolve_to_addrs(host, &addresses)
            }
            Resolved::Nowhere => {
                return Err(format!(
                    "url: cannot resolve {}",
                    url.host_str().unwrap_or_default()
                ));
            }
        },
    };
    builder.build().map_err(|error| format!("url: {error}"))
}

/// 读响应体，读到 `limit` 字节以上就停、算错。
async fn body(mut response: reqwest::Response, limit: usize) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("url: {error}"))?
    {
        if bytes.len() + chunk.len() > limit {
            return Err(format!("url: {}", super::over(limit)));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;
