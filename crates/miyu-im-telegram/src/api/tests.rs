use std::time::Duration;

use serde_json::json;

use super::{Failure, Proxy, read};

#[test]
fn a_reply_that_is_ok_hands_back_its_result() {
    let body = json!({"ok": true, "result": {"id": 7}}).to_string();
    assert_eq!(read(200, body.as_bytes()), Ok(json!({"id": 7})));
}

#[test]
fn refusals_are_told_apart() {
    let refused = |code: u16, extra: serde_json::Value| {
        let mut body = json!({"ok": false, "error_code": code, "description": " Bad Request: chat not found "});
        if let Some(object) = extra.as_object() {
            for (key, value) in object {
                body[key] = value.clone();
            }
        }
        read(code, body.to_string().as_bytes())
    };
    assert_eq!(refused(401, json!({})), Err(Failure::Unauthorized));
    assert_eq!(refused(409, json!({})), Err(Failure::Conflict));
    assert_eq!(
        refused(429, json!({"parameters": {"retry_after": 7}})),
        Err(Failure::Retry(Duration::from_secs(7)))
    );
    // 没说等多久的等一秒，说 0 的也等一秒：不空转。
    assert_eq!(
        refused(429, json!({})),
        Err(Failure::Retry(Duration::from_secs(1)))
    );
    assert_eq!(
        refused(400, json!({})),
        Err(Failure::Failed {
            code: 400,
            description: "Bad Request: chat not found".to_string()
        })
    );
}

#[test]
fn a_reply_that_is_not_json_is_a_network_failure_unless_the_status_says_more() {
    assert_eq!(read(401, b"<html>"), Err(Failure::Unauthorized));
    assert_eq!(read(409, b""), Err(Failure::Conflict));
    assert!(matches!(
        read(502, b"Bad Gateway"),
        Err(Failure::Network(_))
    ));
}

#[test]
fn proxies_read_and_write_back_the_same() {
    for (written, proxy) in [
        ("", Proxy::Environment),
        ("direct", Proxy::Direct),
        (
            "socks5://127.0.0.1:1080",
            Proxy::Address("socks5://127.0.0.1:1080".to_string()),
        ),
    ] {
        assert_eq!(Proxy::parse(written), proxy);
        assert_eq!(proxy.written(), written);
    }
    assert_eq!(Proxy::parse("  direct "), Proxy::Direct);
}

#[test]
fn a_proxy_address_that_cannot_be_read_does_not_build_a_client() {
    let token = miyu_client::settings::Secret::new("1:abc").expect("是字");
    let refused = super::Api::new(
        "https://api.telegram.org",
        &token,
        &Proxy::Address("::not a url::".to_string()),
    );
    let reason = refused.expect_err("读不懂的代理不造");
    assert!(!reason.contains("1:abc"), "原因里不带令牌：{reason}");
}
