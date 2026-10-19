//! 照浏览器的样子用 im 的页面服务（施工 I-5 上）：发一个 HTTP/1.1 请求、读完回应；开通道（WebSocket）。

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use super::{Bridge, within};
use miyu_im::serve::Notice;
use miyu_im::settings::Defaults;

/// `defaults` 里页面服务的端口换成 0，让系统挑（进程里跑的桥都照它，不碰出厂的 8302）。
pub fn any_page_port(defaults: Defaults) -> Defaults {
    Defaults {
        site: serde_json::Map::from_iter([("web_port".to_string(), Value::from(0))]),
        ..defaults
    }
}

/// 读完的一个回应。
#[derive(Debug)]
pub struct Answer {
    /// 状态码。
    pub status: u16,
    /// 头，名字写成小写，照先后。
    pub headers: Vec<(String, String)>,
    /// 正文。
    pub body: Vec<u8>,
}

impl Answer {
    /// 名字是 `name`（小写）的头；没有的是空的。
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// 正文当 JSON 读。
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("正文是 JSON")
    }
}

/// 往本机 `port` 发一个请求：方法 `method`、路径 `path`、`Host` 是 `host`，再带 `headers`（一行一个，不带换行）和正文 `body`。
/// 读到连接关上为止。
pub async fn request(
    port: u16,
    (method, path, host): (&str, &str, &str),
    headers: &[&str],
    body: &[u8],
) -> Answer {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("连得上页面服务");
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    for header in headers {
        head.push_str(header);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).await.expect("写得进");
    stream.write_all(body).await.expect("写得进");
    let mut read = Vec::new();
    within("页面服务回完", stream.read_to_end(&mut read))
        .await
        .expect("读得了");
    let split = read
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("有头");
    let head = String::from_utf8_lossy(&read[..split]).to_string();
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|code| code.parse().ok())
        .expect("有状态行");
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect();
    Answer {
        status,
        headers,
        body: read[split + 4..].to_vec(),
    }
}

/// `GET path`，`Host` 是本机的写法。
pub async fn get(port: u16, path: &str) -> Answer {
    let host = format!("127.0.0.1:{port}");
    request(port, ("GET", path, &host), &[], b"").await
}

/// `POST path`，正文是 JSON `body`，带 `headers`。
pub async fn post(port: u16, path: &str, headers: &[&str], body: &Value) -> Answer {
    let host = format!("127.0.0.1:{port}");
    let mut all = vec!["Content-Type: application/json"];
    all.extend_from_slice(headers);
    request(
        port,
        ("POST", path, &host),
        &all,
        body.to_string().as_bytes(),
    )
    .await
}

/// 开着的通道。
pub type Channel = WebSocketStream<TcpStream>;

/// 开通道（`GET /channel` 升级），带 `Origin` `origin`（空的不带）。
pub async fn channel(port: u16, origin: Option<&str>) -> Channel {
    let stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("连得上页面服务");
    let mut request = format!("ws://127.0.0.1:{port}/channel")
        .into_client_request()
        .expect("网址对");
    if let Some(origin) = origin {
        request
            .headers_mut()
            .insert("origin", origin.parse().expect("是头的值"));
    }
    let (channel, _) = tokio_tungstenite::client_async(request, stream)
        .await
        .expect("升级得成");
    channel
}

/// 发一帧 JSON。
pub async fn say(channel: &mut Channel, message: &Value) {
    channel
        .send(Message::text(message.to_string()))
        .await
        .expect("发得出");
}

/// 下一帧：字的当 JSON 交回 `Ok`，关闭帧交回 `Err(关闭码)`；断了的是 `Err(0)`。
pub async fn heard(channel: &mut Channel) -> Result<Value, u16> {
    within("通道上来了一帧", async {
        loop {
            match channel.next().await {
                Some(Ok(Message::Text(text))) => {
                    return Ok(serde_json::from_str(&text).expect("一帧一条 JSON"));
                }
                Some(Ok(Message::Close(frame))) => {
                    return Err(frame.map_or(0, |frame| u16::from(frame.code)));
                }
                Some(Ok(_)) => {}
                Some(Err(_)) | None => return Err(0),
            }
        }
    })
    .await
}

/// 等桥说页面服务听上了第 `nth` 次（从 1 数），交回端口。
pub async fn page_port(bridge: &Bridge, nth: usize) -> u16 {
    within("页面服务听上", async {
        loop {
            let ports: Vec<u16> = bridge
                .notices
                .lock()
                .expect("没 panic")
                .iter()
                .filter_map(|notice| match notice {
                    Notice::Page { port } => Some(*port),
                    _ => None,
                })
                .collect();
            if let Some(port) = ports.get(nth - 1) {
                return *port;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
}
