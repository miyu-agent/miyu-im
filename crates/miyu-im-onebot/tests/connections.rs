//! 一个连接一个端口（施工 I-81 下，设计 07 第 7.7 节）：连接表写了两个连接，桥两个端口都开，各照各的令牌比，别的连接的令牌进不来；
//! 后台页的 `status` 列出两个连接。推来只剩一个连接的表：另一个端口关掉，已经连着的那一条照样收发。

use serde_json::{Value, json};

use miyu_im::serve::Notice;

use crate::support::fake_core::fake_core;
use crate::support::*;

/// 两个连接的令牌。
const FIRST: &str = "napcat-first";
const SECOND: &str = "napcat-second";

/// 连接表：两个都只听本机、端口 0（让系统挑），令牌各一个。
fn two() -> Value {
    json!({
        "im.qq.listen": 0,
        "im.qq.hosts": ["127.0.0.1", "127.0.0.1"],
        "im.qq.ports": [0, 0],
        "im.qq.tokens": [FIRST, SECOND],
    })
}

/// 桥说过的听上的端口，照先后。
fn listening(bridge: &Bridge) -> Vec<u16> {
    bridge
        .notices
        .lock()
        .expect("没 panic")
        .iter()
        .filter_map(|notice| match notice {
            Notice::Listening { port, .. } => Some(*port),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn each_connection_listens_on_its_own_port_with_its_own_token() {
    let (_dir, root) = temp_root();
    let _core = fake_core(&root);
    let (serve, relay) = serve_handed(root, two(), None);
    let bridge = start(serve).await;
    let ports = within("两个端口都开了", async {
        loop {
            let ports = listening(&bridge);
            if ports.len() >= 2 {
                return ports;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await;
    let (first, second) = (ports[0], ports[1]);
    assert_ne!(first, second);
    assert_eq!(
        napcat(first, "/ws", Auth::Bearer(SECOND), Some(BOT))
            .await
            .err(),
        Some(401),
        "别的连接的令牌进不来"
    );
    assert_eq!(
        napcat(second, "/ws", Auth::Bearer(FIRST), Some(BOT))
            .await
            .err(),
        Some(401)
    );
    let _a = admitted(first, "/ws", Auth::Bearer(FIRST), Some(BOT)).await;
    let mut b = admitted(second, "/ws", Auth::Bearer(SECOND), Some(BOT + 1)).await;
    let status = page_status(&relay, "core-1").await;
    assert_eq!(
        status["connections"],
        json!([
            {"host": "127.0.0.1", "listen": first, "token": "set"},
            {"host": "127.0.0.1", "listen": second, "token": "set"},
        ]),
        "{status}"
    );
    assert_eq!(status["listen"], first, "顶上照头一个连接");

    // 推来只剩第一个连接：第二个端口关掉，连着的那一条不断。
    relay.config(
        json!({"im.qq.hosts": ["127.0.0.1"], "im.qq.ports": [first], "im.qq.tokens": [FIRST]}),
    );
    within("第二个端口关了", async {
        while tokio::net::TcpStream::connect(("127.0.0.1", second))
            .await
            .is_ok()
        {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(b.alive().await, "已经连着的那一条照样连着");
    let status = page_status(&relay, "core-2").await;
    assert_eq!(
        status["connections"].as_array().map(Vec::len),
        Some(1),
        "{status}"
    );
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn without_a_connection_table_the_old_three_settings_are_the_first_connection() {
    // 原来的 `listen`、`token` 照旧当头一个连接（线上的 8301 就这样接着用，不用搬）。
    let (_dir, root) = temp_root();
    let _core = fake_core(&root);
    let (serve, relay) = serve_pushing(root, settings());
    let bridge = start(serve).await;
    let _napcat = admitted(bridge.port, "/ws", Auth::Bearer(TOKEN), Some(BOT)).await;
    let status = page_status(&relay, "core-1").await;
    assert_eq!(
        status["connections"],
        json!([{"host": "127.0.0.1", "listen": bridge.port, "token": "set"}]),
        "{status}"
    );
    bridge.stop().await.expect("停得下");
}
