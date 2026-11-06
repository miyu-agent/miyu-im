//! 认「本机直连的页面」（施工 I-167补）：五条缺一条都不认。

use std::net::SocketAddr;

use hyper::header::{HeaderMap, HeaderName, HeaderValue};

use super::*;

const PORT: u16 = 8302;

/// 本机直连的页面带的头：本机的 Host、同源的 Origin、自定义头。
fn page() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("host", HeaderValue::from_static("127.0.0.1:8302"));
    headers.insert("origin", HeaderValue::from_static("http://127.0.0.1:8302"));
    headers.insert(MARK.0, HeaderValue::from_static(MARK.1));
    headers
}

fn peer(text: &str) -> SocketAddr {
    text.parse().expect("地址")
}

#[test]
fn a_page_opened_on_this_machine_is_local() {
    for from in [
        "127.0.0.1:50000",
        "127.9.9.9:50000",
        "[::1]:50000",
        "[::ffff:127.0.0.1]:50000",
    ] {
        assert!(local_page(peer(from), &page(), PORT), "{from}");
    }
    let mut no_origin = page();
    no_origin.remove("origin");
    assert!(
        local_page(peer("127.0.0.1:1"), &no_origin, PORT),
        "没带 Origin 的也认"
    );
    let mut localhost = page();
    localhost.insert("host", HeaderValue::from_static("localhost:8302"));
    localhost.insert("origin", HeaderValue::from_static("http://localhost:8302"));
    assert!(local_page(peer("127.0.0.1:1"), &localhost, PORT));
}

#[test]
fn a_lan_peer_is_never_local() {
    for from in [
        "192.168.1.9:50000",
        "10.0.0.2:1",
        "[fe80::1]:1",
        "[::ffff:192.168.1.9]:1",
    ] {
        assert!(!local_page(peer(from), &page(), PORT), "{from}");
    }
}

#[test]
fn any_proxy_header_makes_it_not_local() {
    for name in PROXIED {
        let mut proxied = page();
        proxied.insert(
            HeaderName::from_static(name),
            HeaderValue::from_static("192.168.1.9"),
        );
        assert!(!local_page(peer("127.0.0.1:1"), &proxied, PORT), "{name}");
    }
}

#[test]
fn the_host_origin_and_mark_must_all_be_right() {
    let local = peer("127.0.0.1:1");
    let changed = |name: &'static str, value: Option<&'static str>| {
        let mut headers = page();
        match value {
            Some(value) => headers.insert(name, HeaderValue::from_static(value)),
            None => headers.remove(name),
        };
        headers
    };
    let cases = [
        (
            "host",
            Some("192.168.1.9:8302"),
            "写 IP 的 Host（开着局域网时页面服务收，这里不收）",
        ),
        ("host", Some("evil.example:8302"), "别的域名"),
        ("host", Some("127.0.0.1:8303"), "别的端口"),
        ("host", None, "没带 Host"),
        ("origin", Some("http://evil.example"), "别的网站"),
        ("origin", Some("null"), "空来源"),
        (MARK.0, None, "没带自定义头"),
        (MARK.0, Some("yes"), "自定义头的值不对"),
    ];
    for (name, value, why) in cases {
        assert!(!local_page(local, &changed(name, value), PORT), "{why}");
    }
}
