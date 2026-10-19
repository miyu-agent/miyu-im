//! 页面和宿主说话的通道（施工 I-5 上「怎么走」第 7 条）：`GET /channel` 升级成的 WebSocket，一帧一条 JSON。
//!
//! 1. 页面先发 `{"session"}`（登录时交回的会话号；浏览器开 WebSocket 带不了自己的头），[`FIRST`] 以内没发、对不上登录的回关闭帧
//!    4401 `bad_session`。对上了回 `{"account"}`。
//! 2. 之后页面发 `{"id", "method", "params"}`，宿主回 `{"id", "result"}` 或 `{"id", "error": {"reason"}}`（方法在
//!    `methods`）。读不懂的帧记一行调试日志就丢。这一份登录登出了（`POST /logout`），下一条请求回关闭帧 4401。
//! 3. 桥手里的状态变了（`crate::changed`，施工 I-6 上）：推一帧 `{"event": "changed"}`，页面照它再问（不轮询）。

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

use super::{Site, methods};
use crate::TARGET;

/// 第一帧最多等多久。
const FIRST: Duration = Duration::from_secs(10);

/// 会话号不对的关闭码（照 HTTP 的 401，WebSocket 留给程序用的 4000 起）。
const BAD_SESSION: u16 = 4401;

/// 跑一条升级好的通道，直到断开。
pub(super) async fn run<S>(io: S, site: Arc<Site>)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut ws = WebSocketStream::from_raw_socket(io, Role::Server, None).await;
    let session = match tokio::time::timeout(FIRST, text(&mut ws)).await {
        Ok(Some(first)) => serde_json::from_str::<Value>(&first)
            .ok()
            .and_then(|first| first["session"].as_str().map(str::to_string)),
        _ => None,
    };
    let login = session
        .as_deref()
        .and_then(|session| site.logins.get(session));
    let (Some(session), Some(login)) = (session, login) else {
        tracing::info!(target: TARGET, "channel refused");
        close(&mut ws).await;
        return;
    };
    if send(&mut ws, &json!({"account": login.account}))
        .await
        .is_err()
    {
        return;
    }
    let mut changes = site.changed.watch();
    loop {
        let frame = tokio::select! {
            frame = text(&mut ws) => frame,
            changed = changes.changed() => {
                // 桥手里的状态变了（施工 I-6 上）：叫页面再问一遍。叫的一头放下了（桥在停）的不再等它。
                if changed.is_ok() && send(&mut ws, &json!({"event": "changed"})).await.is_err() {
                    return;
                }
                continue;
            }
        };
        let Some(frame) = frame else {
            return;
        };
        let Ok(request) = serde_json::from_str::<Value>(&frame) else {
            tracing::debug!(target: TARGET, "channel frame not json");
            continue;
        };
        let Some(login) = site.logins.get(&session) else {
            close(&mut ws).await;
            return;
        };
        let id = request["id"].clone();
        let reply = match methods::answer(&site, &login.account, request["method"].as_str()) {
            Ok(result) => json!({"id": id, "result": result}),
            Err(reason) => json!({"id": id, "error": {"reason": reason}}),
        };
        if send(&mut ws, &reply).await.is_err() {
            return;
        }
    }
}

/// 读到下一个字的帧；断开、出错的是空的。别的帧（ping 这类 tungstenite 自己回）跳过。
async fn text<S>(ws: &mut WebSocketStream<S>) -> Option<String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    loop {
        match ws.next().await? {
            Ok(Message::Text(text)) => return Some(text.to_string()),
            Ok(Message::Close(_)) | Err(_) => return None,
            Ok(_) => {}
        }
    }
}

/// 发一帧 JSON。
async fn send<S>(
    ws: &mut WebSocketStream<S>,
    message: &Value,
) -> Result<(), tokio_tungstenite::tungstenite::Error>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    ws.send(Message::text(message.to_string())).await
}

/// 回关闭帧 4401 `bad_session`。
async fn close<S>(ws: &mut WebSocketStream<S>)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let frame = CloseFrame {
        code: CloseCode::from(BAD_SESSION),
        reason: "bad_session".into(),
    };
    if let Err(error) = ws.close(Some(frame)).await {
        tracing::debug!(target: TARGET, error = %error, "channel not closed cleanly");
    }
}
