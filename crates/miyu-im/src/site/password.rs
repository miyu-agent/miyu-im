//! 用户名、密码登录（施工 I-5 下，设计 01 第九节第 2 条）：`POST /login/password`，正文 `{"user", "password"}`。照它经本机套接字
//! 握手连核心（`open::Core::connect_password`，核心验），连上的就是这个人的连接，记一份登录，交回 `{"account", "session"}`。
//! 握手换到的登录令牌不留：登录只在内存里，人的连接就是这一条。
//!
//! - 正文不是 JSON、`user`、`password` 不是字：400 `bad_params`。
//! - 用户名或密码不对（`bad_password`）：401 `bad_password`。
//! - 错多了被挡（`login_throttled`）：429 `login_throttled`，带 `retry_after_ms`（核心给了的）。
//! - 核心没在跑、连不上、没回，核心拒了别的：503 `core`，记一行运行日志。
//!
//! 运行日志不记用户名、密码。

use hyper::StatusCode;
use serde_json::{Value, json};

use miyu_client::open;

use super::{HEAD, Site, refused};
use crate::TARGET;

/// 登录的样子：回应的状态和正文。
pub(super) async fn login(site: &Site, body: &[u8]) -> (StatusCode, Value) {
    let given = serde_json::from_slice::<Value>(body).ok();
    let (Some(user), Some(password)) = (
        given.as_ref().and_then(|given| given["user"].as_str()),
        given.as_ref().and_then(|given| given["password"].as_str()),
    ) else {
        return refused(StatusCode::BAD_REQUEST, "bad_params");
    };
    let person = match open::Core::connect_password(&site.root, user, password, HEAD).await {
        Ok((person, _)) => person,
        Err(why) => return turned_away(&why),
    };
    let Some(account) = person.account().map(str::to_string) else {
        tracing::warn!(target: TARGET, "password login without an account");
        return refused(StatusCode::SERVICE_UNAVAILABLE, "core");
    };
    let Some(session) = site
        .logins
        .add(account.clone(), tokio::sync::Mutex::new(person))
    else {
        tracing::warn!(target: TARGET, "no randomness for a session");
        return refused(StatusCode::SERVICE_UNAVAILABLE, "core");
    };
    tracing::info!(target: TARGET, account, "logged in by password");
    (
        StatusCode::OK,
        json!({"account": account, "session": session}),
    )
}

/// 核心不让进的样子（模块的说明）。
fn turned_away(why: &open::Refused) -> (StatusCode, Value) {
    match why.reason.as_str() {
        "bad_password" => {
            tracing::info!(target: TARGET, "password refused");
            refused(StatusCode::UNAUTHORIZED, "bad_password")
        }
        "login_throttled" => {
            tracing::info!(target: TARGET, "password login throttled");
            let mut answer = json!({"reason": "login_throttled"});
            if let Some(wait) = why.retry_after_ms {
                answer["retry_after_ms"] = json!(wait);
            }
            (StatusCode::TOO_MANY_REQUESTS, answer)
        }
        other => {
            tracing::warn!(target: TARGET, reason = other, "password login failed");
            refused(StatusCode::SERVICE_UNAVAILABLE, "core")
        }
    }
}
