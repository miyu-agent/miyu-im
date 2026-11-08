//! 连接表的读、写（施工 I-81 下中）：三个列表照端口那一列算；没写过表的，原来设了 Token 的三项是头一个连接，没设的不算；写回的是
//! 三个列表加上去掉原来的三项。

use serde_json::{Value, json};

use super::{Entry, changes, retired, table, waiting};

/// 照 `config.get` 回答的样子包一层：键到 `{"value": …}`。
fn got(values: Value) -> Value {
    let items: serde_json::Map<String, Value> = values
        .as_object()
        .expect("对象")
        .iter()
        .map(|(key, value)| (format!("im.qq.{key}"), json!({"value": value})))
        .collect();
    json!({"items": items})
}

const FALLBACK: (&str, u64) = ("0.0.0.0", 8301);

#[test]
fn the_table_is_the_three_columns() {
    let read = table(
        &got(json!({
            "hosts": ["127.0.0.1"],
            "ports": [8301, 8302, "bad"],
            "tokens": [{"secret": "a"}, {"secret": "b"}, {"secret": "c"}],
            "listen": 9999,
            "token": {"secret": "old"},
        })),
        "qq",
        FALLBACK,
    );
    assert_eq!(
        read,
        [
            Entry {
                host: "127.0.0.1".into(),
                port: 8301,
                token: json!({"secret": "a"})
            },
            Entry {
                host: "0.0.0.0".into(),
                port: 8302,
                token: json!({"secret": "b"})
            },
        ],
        "地址少了照默认，端口读不出的跳过，原来的三项不看"
    );
}

#[test]
fn without_a_table_the_old_settings_with_a_token_are_the_first_connection() {
    let read = table(
        &got(json!({"token": {"secret": "im-qq-token"}})),
        "qq",
        FALLBACK,
    );
    assert_eq!(
        read,
        [Entry {
            host: "0.0.0.0".into(),
            port: 8301,
            token: json!({"secret": "im-qq-token"})
        }],
        "端口、地址没写的照默认"
    );
    assert!(
        table(&got(json!({"listen": 8301})), "qq", FALLBACK).is_empty(),
        "没设 Token 的不算一个连接"
    );
}

#[test]
fn writing_sets_the_three_columns_and_drops_the_old_settings() {
    let entries = [
        Entry {
            host: "0.0.0.0".into(),
            port: 8301,
            token: json!({"secret": "a"}),
        },
        Entry {
            host: "127.0.0.1".into(),
            port: 8302,
            token: json!({"secret": "b"}),
        },
    ];
    assert_eq!(
        changes("qq", &entries),
        [
            json!({"key": "im.qq.hosts", "value": ["0.0.0.0", "127.0.0.1"]}),
            json!({"key": "im.qq.ports", "value": [8301, 8302]}),
            json!({"key": "im.qq.tokens", "value": [{"secret": "a"}, {"secret": "b"}]}),
            json!({"key": "im.qq.host", "unset": true}),
            json!({"key": "im.qq.listen", "unset": true}),
            json!({"key": "im.qq.token", "unset": true}),
        ]
    );
}

#[test]
fn only_secrets_the_new_table_no_longer_names_are_retired() {
    // 施工 I-118（发布前审查 I4-4）：换了令牌、删了连接，原来的密钥删掉；还在用的、写成别的样子的不动。
    let entry = |port: u64, token: Value| Entry {
        host: "0.0.0.0".to_string(),
        port,
        token,
    };
    let before = [
        entry(8301, json!({"secret": "im-qq-token-8301-1"})),
        entry(8302, json!({"secret": "im-qq-token-8302-1"})),
        entry(8303, json!({"env": "QQ_TOKEN"})),
    ];
    let after = [
        entry(8301, json!({"secret": "im-qq-token-8301-1"})),
        entry(8303, json!({"secret": "im-qq-token-8303-2"})),
    ];
    assert_eq!(retired(&before, &after), ["im-qq-token-8302-1"]);
    assert!(retired(&after, &after).is_empty());
}

#[test]
fn a_port_the_driver_listens_on_without_a_token_is_waiting() {
    // 施工 I-177：配置里写了 Token 的引用、密钥却没有，驱动照没有令牌听着；这个端口新建连接时给它换上 Token，不算重。
    let status = |token: &str| {
        json!({"connections": [{"host": "0.0.0.0", "listen": 8301, "token": token}]})
            .as_object()
            .expect("对象")
            .clone()
    };
    assert!(waiting(&status("none"), 8301));
    assert!(!waiting(&status("set"), 8301));
    assert!(!waiting(&status("none"), 8302));
    assert!(!waiting(&serde_json::Map::new(), 8301));
}
