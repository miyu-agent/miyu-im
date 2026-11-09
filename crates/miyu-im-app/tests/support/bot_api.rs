//! 假的 Telegram Bot API（施工 I-183 中上）：本机回环上一个最小的 HTTP/1 服务端，认 `POST /bot<令牌>/<方法>`，照 [`Mode`] 答
//! `getMe`（能先挂着不答）、`getUpdates`、`getFile`、`sendPhoto`/`sendDocument`（`multipart/form-data`，施工 I-185 中）（照 [`BotApi::file`] 排的，下载走 `GET /file/bot<令牌>/…`），别的方法答 `{"ok": true, "result": true}`。每次调用记下方法和参数（[`BotApi::calls`]）；令牌不对的答
//! 401。`sendMessage` 照 Telegram 回一条消息（编号从 1000 数）。`getUpdates` 有排着的消息就一次交完，没有的挂 100 毫秒答空的（真的挂 50 秒，测试不等那么久）。一个请求一条连接，答完就关。

use std::collections::{HashMap, VecDeque};
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
    /// 取得到的文件：`file_id` → 内容（[`BotApi::file`]，施工 I-185 上）。
    files: HashMap<String, Vec<u8>>,
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
            files: HashMap::new(),
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

    /// 编号是 `id` 的文件内容是 `bytes`（施工 I-185 上）：`getFile` 答 `photos/<编号>.png`，`GET /file/bot<令牌>/…` 交回它。
    pub fn file(&self, id: &str, bytes: Vec<u8>) {
        self.lock().files.insert(id.to_string(), bytes);
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
    let (mut length, mut kind) = (0, String::new());
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).await.unwrap_or(0) == 0 || header == "\r\n" {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            } else if name.eq_ignore_ascii_case("content-type") {
                kind = value.trim().to_string();
            }
        }
    }
    let mut body = vec![0; length];
    if reader.read_exact(&mut body).await.is_err() {
        return;
    }
    let (status, kind, bytes) = match downloaded(&path, &state) {
        Some(found) => found,
        None => {
            let params: Value = match kind.split_once("boundary=") {
                Some((_, boundary)) => form(&body, boundary),
                None => serde_json::from_slice(&body).unwrap_or(Value::Null),
            };
            let (status, reply) = reply(&path, params, &state).await;
            (status, "application/json", reply.to_string().into_bytes())
        }
    };
    let mut response = format!(
        "HTTP/1.1 {status} X\r\ncontent-type: {kind}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        bytes.len()
    )
    .into_bytes();
    response.extend_from_slice(&bytes);
    let mut stream = reader.into_inner();
    if stream.write_all(&response).await.is_ok() {
        // 答完就关：一个请求一条连接。
        drop(stream.shutdown().await);
    }
}

/// 读 `multipart/form-data` 的正文（施工 I-185 中，发图、发文件）：字的格照原样，带文件名的那一格写成 `{"filename", "base64"}`。
fn form(body: &[u8], boundary: &str) -> Value {
    use base64::Engine;
    let mark = format!("--{boundary}");
    let mut fields = serde_json::Map::new();
    let mut rest = body;
    while let Some(at) = find(rest, mark.as_bytes()) {
        rest = &rest[at + mark.len()..];
        let Some(head_end) = find(rest, b"\r\n\r\n") else {
            break;
        };
        let head = String::from_utf8_lossy(&rest[..head_end]).to_string();
        let content = &rest[head_end + 4..];
        let Some(end) = find(content, format!("\r\n{mark}").as_bytes()) else {
            break;
        };
        let value = &content[..end];
        let pick = |key: &str| {
            head.split(&format!("{key}=\""))
                .nth(1)
                .and_then(|tail| tail.split('"').next())
                .map(str::to_string)
        };
        if let Some(name) = pick("name") {
            let value = match pick("filename") {
                Some(filename) => {
                    json!({"filename": filename, "base64": base64::engine::general_purpose::STANDARD.encode(value)})
                }
                None => json!(String::from_utf8_lossy(value)),
            };
            fields.insert(name, value);
        }
        rest = &content[end..];
    }
    Value::Object(fields)
}

/// `needle` 在 `haystack` 里头一次出现的位置。
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// 取文件的路径（`/file/bot<令牌>/photos/<编号>.png`，施工 I-185 上）：HTTP 状态、内容类型、内容；不是取文件的是空的。
fn downloaded(path: &str, state: &Mutex<State>) -> Option<(u16, &'static str, Vec<u8>)> {
    let rest = path.strip_prefix("/file/bot")?;
    let (token, file) = rest.split_once('/')?;
    let id = file.strip_prefix("photos/")?.strip_suffix(".png")?;
    let mut held = state.lock().unwrap_or_else(PoisonError::into_inner);
    held.calls
        .push(("download".to_string(), json!({"path": file})));
    Some(match held.files.get(id) {
        Some(bytes) if token == TG_TOKEN => (200, "image/png", bytes.clone()),
        _ => (404, "text/plain", b"Not Found".to_vec()),
    })
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
    let file_id = params["file_id"].as_str().unwrap_or_default().to_string();
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
        "sendMessage" | "sendPhoto" | "sendDocument" => {
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
        "getFile" => {
            let id = file_id;
            let size = state
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .files
                .get(&id)
                .map(Vec::len);
            match size {
                Some(size) => (
                    200,
                    json!({"ok": true, "result": {"file_id": id, "file_size": size, "file_path": format!("photos/{id}.png")}}),
                ),
                None => (
                    400,
                    json!({"ok": false, "error_code": 400, "description": "Bad Request: invalid file_id"}),
                ),
            }
        }
        _ => (200, json!({"ok": true, "result": true})),
    }
}
