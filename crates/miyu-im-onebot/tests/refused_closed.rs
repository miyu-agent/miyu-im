//! 防御性测试（2026-10-30 发布前第二轮审查）：NapCat 的端口出厂听 `0.0.0.0`（施工 I-27，外人连进来靠令牌挡），令牌不对、路径不对
//! 被拒了的连接，桥回完 401、404 就该关掉，不留着等下一个请求：外人一直开着连接不走，每条占一个任务、一个文件句柄，没有上限，
//! 句柄用光了 NapCat 也连不进来。

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use miyu_session::testkit::Script;

use crate::support::*;

/// 回完以后最多等多久看桥关不关：关的话一下就关。
const CLOSE: Duration = Duration::from_secs(3);

/// 照 WebSocket 升级的样子发一个请求（HTTP/1.1，不写 `Connection: close`），读完回应的头，交回状态行。
async fn refused_once(stream: &mut TcpStream, path: &str, token: &str) -> String {
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {token}\r\n\
         Upgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).await.expect("写得进");
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        let read = within("桥回", stream.read(&mut byte))
            .await
            .expect("读得到");
        assert_ne!(read, 0, "回完头以前就断了");
        head.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    head.lines().next().unwrap_or_default().to_string()
}

#[tokio::test]
async fn a_refused_connection_is_closed_after_the_answer() {
    let home = Home::new(&Script::new([]));
    let bridge = bridge(&home).await;
    for (path, token, status) in [
        ("/onebot/v11/ws", "wrong", "401"),
        ("/not-here", TOKEN, "404"),
    ] {
        let mut stream = TcpStream::connect(("127.0.0.1", bridge.port))
            .await
            .expect("连得上");
        let line = refused_once(&mut stream, path, token).await;
        assert!(line.contains(status), "{path}：{line}");
        // 回应没有内容（`content-length: 0`）：再读，关了的读到头。
        let mut rest = [0u8; 64];
        let after = tokio::time::timeout(CLOSE, stream.read(&mut rest)).await;
        assert!(
            matches!(after, Ok(Ok(0)) | Ok(Err(_))),
            "{path}：被拒了 {status}，{CLOSE:?} 以后连接还开着，桥在等同一条连接上的下一个请求（{after:?}）"
        );
    }
    bridge.stop().await.expect("停得下");
}

/// 读到连接关掉（读到头或者出错）为止，最多等 `wait`：关了的是真。
async fn closed_within(stream: &mut TcpStream, wait: Duration) -> bool {
    let mut rest = [0u8; 64];
    let deadline = tokio::time::Instant::now() + wait;
    loop {
        match tokio::time::timeout_at(deadline, stream.read(&mut rest)).await {
            Ok(Ok(0) | Err(_)) => return true,
            Ok(Ok(_)) => {}
            Err(_) => return false,
        }
    }
}

#[tokio::test]
async fn a_connection_that_never_finishes_its_header_is_closed() {
    // 施工 I-94：请求头 `handshake_seconds` 以内读不完的关掉，不一直占着。
    let home = Home::new(&Script::new([]));
    let bridge = bridge(&home).await;
    let mut stream = TcpStream::connect(("127.0.0.1", bridge.port))
        .await
        .expect("连得上");
    stream
        .write_all(b"GET /onebot/v11/ws HTTP/1.1\r\nHost: 127.0.0.1\r\n")
        .await
        .expect("写得进");
    let limit = crate::support::shipped::tuning().handshake() + Duration::from_secs(5);
    assert!(
        closed_within(&mut stream, limit).await,
        "请求头没写完，{limit:?} 以后还开着"
    );
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn connections_past_the_pending_limit_are_closed_at_once() {
    // 施工 I-94：还没升级的连接满了 `pending_connections` 条，再连进来的当场关掉；NapCat 等前面的让出名额就进得来。
    let home = Home::new(&Script::new([]));
    let bridge = bridge(&home).await;
    let limit = crate::support::shipped::tuning().pending_connections;
    let mut held = Vec::new();
    for _ in 0..limit {
        held.push(
            TcpStream::connect(("127.0.0.1", bridge.port))
                .await
                .expect("连得上"),
        );
    }
    let mut extra = TcpStream::connect(("127.0.0.1", bridge.port))
        .await
        .expect("连得上");
    assert!(
        closed_within(&mut extra, CLOSE).await,
        "名额满了，多的当场关"
    );
    drop(held);
    let _napcat = crate::support::napcat::admitted(
        bridge.port,
        "/onebot/v11/ws",
        crate::support::napcat::Auth::Bearer(TOKEN),
        Some(BOT),
    )
    .await;
    bridge.stop().await.expect("停得下");
}
