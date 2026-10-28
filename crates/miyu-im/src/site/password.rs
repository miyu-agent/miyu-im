//! 访问密码登录（施工 I-5 下，设计 01 第九节第 2 条；施工 I-65 起跟核心 SU-1 只认主人、不带用户名）：`POST /login/password`，
//! 正文 `{"password"}`。照它经本机套接字握手连核心（`open::Core::connect_password`，核心验），连上的就是主人的连接，记一份登录，
//! 交回 `{"session"}`。握手换到的登录令牌不留：登录只在内存里，主人的连接就是这一条。
//!
//! - 正文不是 JSON、`password` 不是字：400 `bad_params`。
//! - 密码不对（`bad_password`）：401 `bad_password`。
//! - 还没设访问密码（`password_not_set`）：409 `password_not_set`。
//! - 错多了被挡（`login_throttled`）：429 `login_throttled`，带 `retry_after_ms`（核心给了的）。
//! - 核心没在跑、连不上、没回，核心拒了别的：503 `core`，记一行运行日志。
//!
//! 运行日志不记密码。

use hyper::StatusCode;
use serde_json::{Value, json};

use miyu_client::open;

use super::{HEAD, Site, refused};
use crate::TARGET;

/// 登录的样子：回应的状态和正文。
pub(super) async fn login(site: &Site, body: &[u8]) -> (StatusCode, Value) {
    let given = serde_json::from_slice::<Value>(body).ok();
    let Some(password) = given.as_ref().and_then(|given| given["password"].as_str()) else {
        return refused(StatusCode::BAD_REQUEST, "bad_params");
    };
    let person = match open::Core::connect_password(&site.root, password, HEAD).await {
        Ok((person, _)) => person,
        Err(why) => return turned_away(&why),
    };
    let Some(session) = site.logins.add(tokio::sync::Mutex::new(person)) else {
        tracing::warn!(target: TARGET, "no randomness for a session");
        return refused(StatusCode::SERVICE_UNAVAILABLE, "core");
    };
    tracing::info!(target: TARGET, "logged in by password");
    (StatusCode::OK, json!({"session": session}))
}

/// 核心不让进的样子（模块的说明）。
fn turned_away(why: &open::Refused) -> (StatusCode, Value) {
    match why.reason.as_str() {
        "bad_password" => {
            tracing::info!(target: TARGET, "password refused");
            refused(StatusCode::UNAUTHORIZED, "bad_password")
        }
        "password_not_set" => {
            tracing::info!(target: TARGET, "password login before a password is set");
            refused(StatusCode::CONFLICT, "password_not_set")
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
