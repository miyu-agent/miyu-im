//! 页面服务的一个请求（施工 I-5 上「怎么走」第 3 到 7 条）：先看地址（[`super::guard`]），再照方法和路径分：
//!
//! - `GET /channel`：WebSocket 的升级请求回 101，升级交进 `slot`（[`super::channel`] 接着跑）；不是升级请求的 400。
//! - `POST /login/ticket`：交接票换登录（[`super::ticket`]）。`POST /login/password`：用户名、密码登录（[`super::password`]）。
//!   正文最多 [`BODY`] 字节，多了 413。
//! - `POST /logout`：`Authorization: Bearer <会话号>` 的那份登录拿掉，在它的人的连接上作废登录令牌（`account.logout`，不成的记一行
//!   运行日志），回 204（没有这一份也是）。
//! - `GET` 别的路径：页面的文件（[`super::files`]），没有的 404。
//! - 别的方法：405。
//!
//! 回应都带 `Cache-Control: no-store`、`X-Content-Type-Options: nosniff`、只许同源的 `Content-Security-Policy`（不许被别的页面
//! 放进框里）。

use std::sync::{Mutex, PoisonError};

use http_body_util::{BodyExt, Full, Limited};
use hyper::body::{Bytes, Incoming};
use hyper::header::{self, HeaderValue};
use hyper::upgrade::OnUpgrade;
use hyper::{Method, Request, Response, StatusCode};
use serde_json::Value;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;

use super::{Site, files, guard, password, ticket};
use crate::TARGET;

/// 登录的正文最多多少字节。
const BODY: usize = 4096;

/// 同源的内容安全策略。
const POLICY: &str = "default-src 'self'; frame-ancestors 'none'";

/// 一个请求的回应；升级成的把升级交进 `slot`。`port` 是这个连接进来的端口。
pub(super) async fn handle(
    mut request: Request<Incoming>,
    port: u16,
    site: &Site,
    slot: &Mutex<Option<OnUpgrade>>,
) -> Response<Full<Bytes>> {
    let headers = request.headers();
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok());
    if !guard::local_host(host, port) {
        tracing::warn!(target: TARGET, host, "page request for another host");
        return empty(StatusCode::MISDIRECTED_REQUEST);
    }
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    if !guard::same_origin(origin, host.unwrap_or_default()) {
        tracing::warn!(target: TARGET, origin, "page request from another origin");
        return empty(StatusCode::FORBIDDEN);
    }
    let path = request.uri().path().to_string();
    match (request.method().clone(), path.as_str()) {
        (Method::GET, "/channel") => upgrade(&mut request, slot),
        (Method::POST, "/login/ticket") => match body(request).await {
            Some(body) => {
                let (status, answer) = ticket::redeem(site, &body).await;
                json(status, &answer)
            }
            None => empty(StatusCode::PAYLOAD_TOO_LARGE),
        },
        (Method::POST, "/login/password") => match body(request).await {
            Some(body) => {
                let (status, answer) = password::login(site, &body).await;
                json(status, &answer)
            }
            None => empty(StatusCode::PAYLOAD_TOO_LARGE),
        },
        (Method::POST, "/logout") => {
            if let Some(login) = bearer(&request).and_then(|session| site.logins.remove(session)) {
                let revoked = login
                    .person
                    .lock()
                    .await
                    .call("logout", "account.logout", serde_json::json!({}))
                    .await;
                match revoked {
                    Ok(_) => tracing::info!(target: TARGET, "logged out"),
                    Err(why) => {
                        tracing::warn!(target: TARGET, reason = why.reason, "logged out, login not revoked")
                    }
                }
            }
            empty(StatusCode::NO_CONTENT)
        }
        (Method::GET, path) => match files::name(path) {
            Some(name) => match files::read(&site.files, name).await {
                Some(bytes) => file(bytes, files::kind(name)),
                None => empty(StatusCode::NOT_FOUND),
            },
            None => empty(StatusCode::NOT_FOUND),
        },
        _ => empty(StatusCode::METHOD_NOT_ALLOWED),
    }
}

/// WebSocket 的升级请求：回 101、带上算好的 `Sec-WebSocket-Accept`，把升级交进 `slot`。不是升级请求的 400。
fn upgrade(
    request: &mut Request<Incoming>,
    slot: &Mutex<Option<OnUpgrade>>,
) -> Response<Full<Bytes>> {
    let headers = request.headers();
    let upgrading = headers
        .get(header::UPGRADE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("websocket"));
    let version = headers
        .get(header::SEC_WEBSOCKET_VERSION)
        .is_some_and(|value| value.as_bytes() == b"13");
    let Some(accept) = headers
        .get(header::SEC_WEBSOCKET_KEY)
        .filter(|_| upgrading && version)
        .and_then(|key| HeaderValue::from_str(&derive_accept_key(key.as_bytes())).ok())
    else {
        return empty(StatusCode::BAD_REQUEST);
    };
    *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(hyper::upgrade::on(request));
    let mut response = empty(StatusCode::SWITCHING_PROTOCOLS);
    let headers = response.headers_mut();
    headers.insert(header::UPGRADE, HeaderValue::from_static("websocket"));
    headers.insert(header::CONNECTION, HeaderValue::from_static("Upgrade"));
    headers.insert(header::SEC_WEBSOCKET_ACCEPT, accept);
    response
}

/// 读完正文，最多 [`BODY`] 字节；多了、读不完的是空的。
async fn body(request: Request<Incoming>) -> Option<Bytes> {
    Limited::new(request.into_body(), BODY)
        .collect()
        .await
        .ok()
        .map(http_body_util::Collected::to_bytes)
}

/// `Authorization: Bearer <会话号>` 里的会话号。
fn bearer(request: &Request<Incoming>) -> Option<&str> {
    request
        .headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
}

/// 正文是 JSON 的回应。
fn json(status: StatusCode, answer: &Value) -> Response<Full<Bytes>> {
    let mut response = with(status, Bytes::from(answer.to_string()));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    response
}

/// 页面的一个文件。
fn file(bytes: Vec<u8>, kind: &'static str) -> Response<Full<Bytes>> {
    let mut response = with(StatusCode::OK, Bytes::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(kind));
    response
}

/// 没有正文的回应。
fn empty(status: StatusCode) -> Response<Full<Bytes>> {
    with(status, Bytes::new())
}

/// 状态 `status`、正文 `bytes`，带上每个回应都带的几个头。
fn with(status: StatusCode, bytes: Bytes) -> Response<Full<Bytes>> {
    let mut response = Response::new(Full::new(bytes));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(POLICY),
    );
    response
}
