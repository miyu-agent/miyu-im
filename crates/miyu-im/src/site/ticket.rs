//! 交接票换登录（施工 I-5 上「怎么走」第 5 条，设计 01 第九节第 3 条）：`POST /login/ticket`，正文 `{"ticket"}`。在扩展连接上
//! 调核心的 `login.redeem` 换登录令牌，照它经本机套接字另开一条人的连接（`open::Core::connect_login`），记一份登录，交回
//! `{"account", "session"}`。
//!
//! - 正文不是 JSON、没有 `ticket`：400 `bad_params`。
//! - 核心说票不对（没有、过期、用过，`bad_ticket`）：401 `bad_ticket`。
//! - 扩展连接断了、核心拒了别的、人的连接连不上、取不到随机数：503 `core`，记一行运行日志。
//!
//! 登录令牌只在这里过一下手：交给人的连接握手以后不留。

use hyper::StatusCode;
use serde_json::{Value, json};

use miyu_client::open;

use super::Site;
use crate::TARGET;
use crate::core::reason;

/// 换出来的样子：回应的状态和正文。
pub(super) async fn redeem(site: &Site, body: &[u8]) -> (StatusCode, Value) {
    let ticket = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|body| body["ticket"].as_str().map(str::to_string));
    let Some(ticket) = ticket else {
        return refused(StatusCode::BAD_REQUEST, "bad_params");
    };
    let Ok(reply) = site
        .caller
        .call("login.redeem", json!({"ticket": ticket}))
        .await
    else {
        tracing::warn!(target: TARGET, "ticket not redeemed: core gone");
        return refused(StatusCode::SERVICE_UNAVAILABLE, "core");
    };
    match reason(&reply) {
        None => {}
        Some("bad_ticket") => {
            tracing::info!(target: TARGET, "ticket refused");
            return refused(StatusCode::UNAUTHORIZED, "bad_ticket");
        }
        Some(other) => {
            tracing::warn!(target: TARGET, reason = other, "ticket not redeemed");
            return refused(StatusCode::SERVICE_UNAVAILABLE, "core");
        }
    }
    let result = &reply["result"];
    let (Some(account), Some(token)) = (
        result["account"].as_str(),
        result["login"]["token"].as_str(),
    ) else {
        tracing::warn!(target: TARGET, "redeemed login not understood");
        return refused(StatusCode::SERVICE_UNAVAILABLE, "core");
    };
    let person = match open::Core::connect_login(&site.root, token, HEAD).await {
        Ok(person) => person,
        Err(error) => {
            tracing::warn!(target: TARGET, error = %error, "person not connected");
            return refused(StatusCode::SERVICE_UNAVAILABLE, "core");
        }
    };
    let Some(session) = site
        .logins
        .add(account.to_string(), tokio::sync::Mutex::new(person))
    else {
        tracing::warn!(target: TARGET, "no randomness for a session");
        return refused(StatusCode::SERVICE_UNAVAILABLE, "core");
    };
    tracing::info!(target: TARGET, account, "logged in by ticket");
    (
        StatusCode::OK,
        json!({"account": account, "session": session}),
    )
}

/// 人的连接握手时报的头（只记进核心的运行日志）。
const HEAD: &str = "im-page";

/// 不成的回应：状态 `status`，正文 `{"reason"}`。
fn refused(status: StatusCode, reason: &str) -> (StatusCode, Value) {
    (status, json!({"reason": reason}))
}
