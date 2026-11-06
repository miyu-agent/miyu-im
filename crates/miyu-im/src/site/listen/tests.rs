//! 听哪个地址（施工 I-167）：只认两种写法，别的照本机；只听本机的没有局域网地址。

use std::net::{IpAddr, Ipv4Addr};

use serde_json::{Map, Value, json};

use super::*;

fn config(listen: Value) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert(LISTEN.to_string(), listen);
    map
}

#[test]
fn only_the_two_spellings_are_taken() {
    assert_eq!(
        wanted(&config(json!("0.0.0.0"))),
        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    );
    for other in [
        json!("127.0.0.1"),
        json!("192.168.1.2"),
        json!(null),
        json!(8),
    ] {
        assert_eq!(
            wanted(&config(other.clone())),
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            "{other}"
        );
    }
    assert_eq!(wanted(&Map::new()), IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert!(valid("0.0.0.0") && valid("127.0.0.1"));
    assert!(!valid("192.168.1.2") && !valid(""));
}

#[test]
fn a_local_listener_has_no_lan_address_and_a_fixed_one_is_itself() {
    assert_eq!(lan_address(IpAddr::V4(Ipv4Addr::LOCALHOST)), None);
    let fixed = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 9));
    assert_eq!(lan_address(fixed), Some(fixed));
}
