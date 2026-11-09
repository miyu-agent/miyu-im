use serde_json::{Map, Value, json};

use super::{OFFICIAL, read};
use crate::api::Proxy;

fn config(value: Value) -> Map<String, Value> {
    value.as_object().expect("是对象").clone()
}

#[test]
fn nothing_written_is_no_connection_on_the_official_api() {
    let (all, api) = read(&Map::new(), |_| None);
    assert!(all.is_empty());
    assert_eq!(api, OFFICIAL);
}

#[test]
fn each_token_is_a_connection_with_its_own_proxy() {
    let (all, api) = read(
        &config(
            json!({"tokens": ["1:a", "2:b", " ", 3], "proxies": ["direct", "http://127.0.0.1:8080"], "api": "http://127.0.0.1:9/"}),
        ),
        |_| None,
    );
    assert_eq!(api, "http://127.0.0.1:9/");
    assert_eq!(all.len(), 4);
    assert_eq!(all[0].proxy, Proxy::Direct);
    assert_eq!(
        all[1].proxy,
        Proxy::Address("http://127.0.0.1:8080".to_string())
    );
    // 空白的、不是字的令牌照旧占一个（报「没有令牌」），代理少了的跟随联网设置。
    assert!(all[2].token.is_none());
    assert!(all[3].token.is_none());
    assert_eq!(all[3].proxy, Proxy::Environment);
    assert_eq!(
        all.iter().map(|one| one.index).collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
}

#[test]
fn the_same_token_twice_is_one_connection() {
    let (all, _) = read(&config(json!({"tokens": ["1:a", "1:a"]})), |_| None);
    assert_eq!(all.len(), 1);
}

#[test]
fn an_api_address_that_is_not_http_falls_back_to_the_default() {
    let (_, api) = read(&config(json!({"api": "ftp://x"})), |name| {
        (name == "api").then(|| json!("https://example.invalid"))
    });
    assert_eq!(api, "https://example.invalid");
}
