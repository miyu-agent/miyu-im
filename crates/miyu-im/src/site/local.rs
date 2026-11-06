//! 本机打开的页面不要密码（设计 11 第一节第 2 条，施工 I-167补，核心 LP-1、LP-2）：`POST /login/local`。认得是本机直连的页面，就在扩展
//! 连接上向核心要一张「本机页面」的交接票（`login.ticket {"via": "page"}`），照交接票换登录（[`super::ticket::redeem_ticket`]），
//! 交回 `{"session"}`。核心挡不住这一道（它只看得到 im 的扩展连接），认「本机」全靠这里，照核心参考 extensions 写给扩展作者的义务：
//!
//! 1. 对端是回环地址（`127.0.0.0/8`、`::1`，IPv4 映射的也认）。局域网来的一律走密码（`/login/password`）。
//! 2. 不带任何反向代理头（[`PROXIED`]）：同机的反向代理转来的请求对端也是回环，靠这个分出来。
//! 3. `Host` 是本机的写法加监听的端口（[`super::guard::local_host`]）：开着局域网时写 IP 的 `Host` 页面服务也收，这里不收。
//! 4. `Origin` 没带，或者是 `http://` 加同一个 `Host`。
//! 5. 带自定义头 `X-Miyu-Page: local`（同网页软件的 `/ticket`）：跨站的表单、简单请求带不上自定义头。
//!
//! 不是本机的回 403 `not_local`；核心说 im 没批 `login.page` 能力的回 403 `needs_approval`；别的照交接票的。票、登录令牌都不进日志。
//!
//! 挡不住的一种（核心的会话审过、写明）：同机手配的反向代理把局域网的请求转到 `127.0.0.1:<端口>`，把 `Host` 改写成本机（nginx 的
//! `proxy_pass` 默认就这样），又一个转发头都不加，局域网里一个不是浏览器的客户端自己带上 `X-Miyu-Page`、不带 `Origin`，就被认成本机。
//! 浏览器发的请求会带 `Origin`，挡得住。所以别把 im 放在反向代理后面：要从局域网用，开 im 自己的局域网开关（设置页「访问」）。

use std::net::{IpAddr, SocketAddr};

use hyper::StatusCode;
use hyper::header::HeaderMap;
use serde_json::{Value, json};

use super::guard::{local_host, same_origin};
use super::{Site, refused, ticket};
use crate::TARGET;
use crate::core::reason;

/// 反向代理会加的头：带了其中任何一个就不是本机直连（核心的会话审 I-167补 时补了 CDN、负载均衡常用的几种）。
pub(super) const PROXIED: [&str; 15] = [
    "forwarded",
    "x-forwarded-for",
    "x-forwarded-host",
    "x-forwarded-proto",
    "x-forwarded-port",
    "x-forwarded-server",
    "x-forwarded-ssl",
    "x-original-forwarded-for",
    "x-real-ip",
    "x-client-ip",
    "x-cluster-client-ip",
    "cf-connecting-ip",
    "fastly-client-ip",
    "true-client-ip",
    "via",
];

/// 页面要带的自定义头和值。
pub(super) const MARK: (&str, &str) = ("x-miyu-page", "local");

/// 这个请求是不是本机直连的页面发的（模块的说明五条都对上）：对端 `peer`，请求头 `headers`，监听的端口 `port`。纯逻辑。
pub(super) fn local_page(peer: SocketAddr, headers: &HeaderMap, port: u16) -> bool {
    let text = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    let loopback = match peer.ip() {
        IpAddr::V4(ip) => ip.is_loopback(),
        IpAddr::V6(ip) => {
            ip.is_loopback() || ip.to_ipv4_mapped().is_some_and(|ip| ip.is_loopback())
        }
    };
    let host = text("host");
    loopback
        && PROXIED.iter().all(|name| !headers.contains_key(*name))
        && local_host(host, port)
        && same_origin(text("origin"), host.unwrap_or_default())
        && text(MARK.0) == Some(MARK.1)
}

/// `POST /login/local`：认过是本机直连的页面以后，要一张本机页面的票换登录。
pub(super) async fn login(site: &Site, local: bool) -> (StatusCode, Value) {
    if !local {
        tracing::info!(target: TARGET, "local login refused: not a local page");
        return refused(StatusCode::FORBIDDEN, "not_local");
    }
    let Ok(reply) = site
        .caller
        .call("login.ticket", json!({"via": "page"}))
        .await
    else {
        tracing::warn!(target: TARGET, "local ticket not asked: core gone");
        return refused(StatusCode::SERVICE_UNAVAILABLE, "core");
    };
    match reason(&reply) {
        None => {}
        Some("needs_approval") => {
            tracing::warn!(target: TARGET, "local login needs the login.page capability approved");
            return refused(StatusCode::FORBIDDEN, "needs_approval");
        }
        Some(other) => {
            tracing::warn!(target: TARGET, reason = other, "local ticket refused");
            return refused(StatusCode::SERVICE_UNAVAILABLE, "core");
        }
    }
    let Some(ticket) = reply["result"]["ticket"].as_str() else {
        tracing::warn!(target: TARGET, "local ticket not understood");
        return refused(StatusCode::SERVICE_UNAVAILABLE, "core");
    };
    let (status, answer) = ticket::redeem_ticket(site, ticket).await;
    if status == StatusCode::OK {
        tracing::info!(target: TARGET, "logged in as a local page");
    }
    (status, answer)
}

#[cfg(test)]
mod tests;
