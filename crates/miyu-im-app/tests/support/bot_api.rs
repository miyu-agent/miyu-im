//! 假的 Telegram Bot API（施工 I-183 中上）：本机回环上一个最小的 HTTP/1 服务端，认 `POST /bot<令牌>/<方法>`，照 [`Mode`] 答
//! `getMe`（能先挂着不答）、`getUpdates`，别的方法答 `{"ok": true, "result": true}`。每次调用记下方法和参数（[`BotApi::calls`]）；令牌不对的答
//! 401。`sendMessage` 照 Telegram 回一条消息（编号从 1000 数）。`getUpdates` 有排着的消息就一次交完，没有的挂 100 毫秒答空的（真的挂 50 秒，测试不等那么久）。一个请求一条连接，答完就关。

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use super::TG_TOKEN;

/// 机器人的号。
pub const BOT_ID: i64 = 111;

/// 怎么答。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// 照常。
    Normal,
    /// `getUpdates` 答 409（这个令牌别处也在收）。
    Conflict,
    /// 都答 401（令牌被吊销了）。
    Unauthorized,
}

#[derive(Debug)]
struct State {
    mode: Mode,
    updates: VecDeque<Value>,
    calls: Vec<(String, Value)>,
    /// `getMe` 先挂着不答（[`BotApi::hold`]）。
    held: bool,
}

/// 一个跑着的假 Bot API。
#[derive(Debug, Clone)]
pub struct BotApi {
    /// 地址：配置 `tg.api` 写它。
    pub url: String,
    state: Arc<Mutex<State>>,
}

impl BotApi {
    /// 起一个，照 `mode` 答。
    pub async fn start(mode: Mode) -> BotApi {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("听得上");
        let url = format!("http://{}", listener.local_addr().expect("有地址"));
        let state = Arc::new(Mutex::new(State {
            mode,
            updates: VecDeque::new(),
            calls: Vec::new(),
            held: false,
        }));
        let serving = Arc::clone(&state);
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(answer(stream, Arc::clone(&serving)));
            }
        });
        BotApi { url, state }
    }

    /// 排一条更新，下一次 `getUpdates` 交出去。
    pub fn push(&self, update: Value) {
        self.lock().updates.push_back(update);
    }

    /// `getMe` 挂着不答（`true`）、放开（`false`）：摆出「连接重连了、还没认出机器人」的那一阵。
    pub fn hold(&self, held: bool) {
        self.lock().held = held;
    }

    /// 到现在的调用：方法和参数。
    pub fn calls(&self) -> Vec<(String, Value)> {
        self.lock().calls.clone()
    }

    /// 等到有一次调用合 `wanted`（最多 30 秒）：交回那一次的参数。
    pub async fn until(&self, wanted: impl Fn(&str, &Value) -> bool) -> Value {
        self.until_from(0, wanted).await
    }

    /// 同 [`BotApi::until`]，只看第 `from` 次（从 0 数）以后的调用。
    pub async fn until_from(&self, from: usize, wanted: impl Fn(&str, &Value) -> bool) -> Value {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            if let Some((_, params)) = self
                .calls()
                .into_iter()
                .skip(from)
                .find(|(method, params)| wanted(method, params))
            {
                return params;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "等不到那一次调用：{:?}",
                self.calls()
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// 答一个请求。
async fn answer(stream: TcpStream, state: Arc<Mutex<State>>) {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).await.is_err() {
        return;
    }
    let path = line
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_string();
    let mut length = 0;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).await.unwrap_or(0) == 0 || header == "\r\n" {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0; length];
    if reader.read_exact(&mut body).await.is_err() {
        return;
    }
    let params: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let (status, reply) = reply(&path, params, &state).await;
    let text = reply.to_string();
    let response = format!(
        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}",
        text.len()
    );
    let mut stream = reader.into_inner();
    if stream.write_all(response.as_bytes()).await.is_ok() {
        // 答完就关：一个请求一条连接。
        drop(stream.shutdown().await);
    }
}

/// 照路径、参数答：HTTP 状态和正文。
async fn reply(path: &str, params: Value, state: &Mutex<State>) -> (u16, Value) {
    let Some((token, method)) = path
        .strip_prefix("/bot")
        .and_then(|rest| rest.split_once('/'))
    else {
        return (
            404,
            json!({"ok": false, "error_code": 404, "description": "Not Found"}),
        );
    };
    let mode = {
        let mut held = state.lock().unwrap_or_else(PoisonError::into_inner);
        held.calls.push((method.to_string(), params));
        held.mode
    };
    if token != TG_TOKEN || mode == Mode::Unauthorized {
        return (
            401,
            json!({"ok": false, "error_code": 401, "description": "Unauthorized"}),
        );
    }
    while method == "getMe" && state.lock().unwrap_or_else(PoisonError::into_inner).held {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    match method {
        "getMe" => (
            200,
            json!({"ok": true, "result": {"id": BOT_ID, "is_bot": true, "first_name": "Miyu", "last_name": "Bot", "username": "miyu_test_bot", "can_read_all_group_messages": false}}),
        ),
        "getUpdates" if mode == Mode::Conflict => (
            409,
            json!({"ok": false, "error_code": 409, "description": "Conflict: terminated by other getUpdates request"}),
        ),
        "getUpdates" => {
            let ready: Vec<Value> = state
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .updates
                .drain(..)
                .collect();
            if ready.is_empty() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            (200, json!({"ok": true, "result": ready}))
        }
        // 发出去的：照 Telegram 回一条消息，编号从 1000 数。
        "sendMessage" => {
            let sent = state
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .calls
                .len();
            (
                200,
                json!({"ok": true, "result": {"message_id": 1000 + sent, "date": 1, "chat": {"id": 0}}}),
            )
        }
        _ => (200, json!({"ok": true, "result": true})),
    }
}
