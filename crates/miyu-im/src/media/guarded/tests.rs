//! 带闸的下载（施工 I-96）：本机的假服务器照测试的口子当公网（名字只照表解析），内网的地址、跳到内网的、跳太多的、太大的都不下。

use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use miyu_client::http::{Guard, Proxies, Testing};
// 回应的样子门面没转出：测试照开发依赖直接拿（开发依赖不算桥依赖核心的 crate，`tests/dependencies.rs`）。
use miyu_http::testkit::{Piece, Reply, Server};

use super::{HOPS, download};

/// 一份 `status` 的回应，带着头、正文是 `body`。
fn reply(status: u16, headers: &[(&str, &str)], body: &[u8]) -> Reply {
    Reply {
        status,
        headers: headers
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect(),
        body: vec![Piece::Bytes(body.to_vec())],
    }
}

/// 假服务器的端口。
fn port(server: &Server) -> u16 {
    server
        .base_url
        .trim_start_matches("http://127.0.0.1:")
        .split('/')
        .next()
        .and_then(|port| port.parse().ok())
        .expect("有端口")
}

/// 闸：`cdn.test` 解析到本机（当公网）、`inner.test` 解析到内网。
fn guard() -> Guard {
    Guard::testing(Testing {
        loopback_public: true,
        hosts: vec![
            ("cdn.test".to_string(), IpAddr::V4(Ipv4Addr::LOCALHOST)),
            (
                "inner.test".to_string(),
                IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            ),
        ],
        proxy: None,
        no_proxy: String::new(),
    })
}

const WAIT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn a_public_address_is_downloaded() {
    let server = Server::start(vec![reply(200, &[], b"picture")]).await;
    let url = format!("http://cdn.test:{}/a.jpg", port(&server));
    let got = download(&url, (&guard(), &Proxies::given(None, "")), WAIT, 1024).await;
    assert_eq!(got.as_deref(), Ok(&b"picture"[..]));
}

#[tokio::test]
async fn an_inner_address_and_a_redirect_into_one_are_refused() {
    let proxies = Proxies::given(None, "");
    let inner = download("http://inner.test/a.jpg", (&guard(), &proxies), WAIT, 1024).await;
    assert!(inner.is_err_and(|why| why.contains("not a public address")));
    // 先到公网的那一跳，跳到内网的不跟。
    let server = Server::start(vec![reply(
        302,
        &[("Location", "http://inner.test/b.jpg")],
        b"",
    )])
    .await;
    let url = format!("http://cdn.test:{}/a.jpg", port(&server));
    let moved = download(&url, (&guard(), &proxies), WAIT, 1024).await;
    assert!(moved.is_err_and(|why| why.contains("not a public address")));
    // 主机直接写回环 IP、闸不把回环当公网的：也不下。
    let strict = Guard::testing(Testing::default());
    let looped = download(&url, (&strict, &proxies), WAIT, 1024).await;
    assert!(looped.is_err());
}

#[tokio::test]
async fn redirects_are_followed_a_few_times_then_stop() {
    let proxies = Proxies::given(None, "");
    let server = Server::start(vec![reply(302, &[("Location", "/again")], b""); HOPS + 1]).await;
    let url = format!("http://cdn.test:{}/a.jpg", port(&server));
    let looping = download(&url, (&guard(), &proxies), WAIT, 1024).await;
    assert_eq!(looping, Err("url: too many redirects".to_string()));
    let server = Server::start(vec![
        reply(301, &[("Location", "/real.jpg")], b""),
        reply(200, &[], b"real"),
    ])
    .await;
    let url = format!("http://cdn.test:{}/a.jpg", port(&server));
    let followed = download(&url, (&guard(), &proxies), WAIT, 1024).await;
    assert_eq!(followed.as_deref(), Ok(&b"real"[..]));
}

#[tokio::test]
async fn a_body_over_the_limit_is_refused() {
    let server = Server::start(vec![reply(200, &[], &[7u8; 2048])]).await;
    let url = format!("http://cdn.test:{}/a.jpg", port(&server));
    let got = download(&url, (&guard(), &Proxies::given(None, "")), WAIT, 1024).await;
    assert!(got.is_err_and(|why| why.starts_with("url: over")));
}
