//! 发布前第四轮审查（2026-11-02）的防御性测试：现在是红的，修好了变绿。
//!
//! NapCat 的端口出厂听 `0.0.0.0`（施工 I-27）。施工 I-94 给还没升级的连接设了名额（`pending_connections`，出厂 64），满了的新连接
//! 当场关；但名额是所有来源共用的一份：一台别的机器开满 64 条空连接、到时被关了马上再开，NapCat 连进来的那一条总是被当场关掉，
//! 断过一次就一直连不回来。不要令牌，谁连得到端口谁就做得到。
//!
//! 另一条：后台页新建、改连接时地址收任何不空的字（`site/connections.rs`），连接表里写不成 IP 的（`localhost` 这类）驱动照默认
//! 读成 `0.0.0.0`（`settings.rs` 的 `host`）：人想只给本机，桥却对所有网卡开着。

#[cfg(target_os = "linux")]
use std::time::Duration;

#[cfg(target_os = "linux")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(target_os = "linux")]
use tokio::net::{TcpSocket, TcpStream};

#[cfg(target_os = "linux")]
use miyu_session::testkit::Script;

#[cfg(target_os = "linux")]
use crate::support::*;

#[cfg(target_os = "linux")]
/// 从 `source` 这个本机地址连 `port`（Linux 上整个 `127.0.0.0/8` 都是回环，能挑来源地址）。
async fn from(source: [u8; 4], port: u16) -> TcpStream {
    let socket = TcpSocket::new_v4().expect("建得了");
    socket
        .bind((std::net::Ipv4Addr::from(source), 0).into())
        .expect("绑得上");
    socket
        .connect((std::net::Ipv4Addr::LOCALHOST, port).into())
        .await
        .expect("连得上")
}

#[cfg(target_os = "linux")]
/// NapCat 照 WebSocket 升级的样子带对的令牌来一次：交回状态行（被当场关掉的是空的）。
async fn napcat_once(port: u16) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("连得上");
    let request = format!(
        "GET /onebot/v11/ws HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {TOKEN}\r\n\
         X-Self-ID: {BOT}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n"
    );
    if stream.write_all(request.as_bytes()).await.is_err() {
        return String::new();
    }
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match tokio::time::timeout(Duration::from_secs(5), stream.read(&mut byte)).await {
            Ok(Ok(1)) => head.push(byte[0]),
            _ => break,
        }
    }
    String::from_utf8_lossy(&head)
        .lines()
        .next()
        .unwrap_or_default()
        .to_string()
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn strangers_holding_every_pending_slot_do_not_lock_napcat_out() {
    let home = Home::new(&Script::new([]));
    let bridge = bridge(&home).await;
    let limit = crate::support::shipped::tuning().pending_connections;
    // 别的机器（来源 127.0.0.2）开满名额，什么都不发
    let mut held = Vec::new();
    for _ in 0..limit {
        held.push(from([127, 0, 0, 2], bridge.port).await);
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    // NapCat（来源 127.0.0.1）带着对的令牌来：该进得来
    let line = napcat_once(bridge.port).await;
    assert!(
        line.contains("101"),
        "别的来源占满了 {limit} 个没升级的名额，带对令牌的 NapCat 被当场关掉，连不进来（读到的状态行：{line:?}）"
    );
    drop(held);
    bridge.stop().await.expect("停得下");
}
