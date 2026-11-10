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
    assert!(lan_addresses(IpAddr::V4(Ipv4Addr::LOCALHOST)).is_empty());
    let fixed = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 9));
    assert_eq!(lan_addresses(fixed), [(None, fixed)]);
}

fn v4(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(a, b, c, d))
}

#[test]
fn every_card_is_listed_with_its_name() {
    // 施工 I-193（照网页软件 W-1135，项目主人 11-10）：开着代理 TUN 的 Mac，往外的那个是 198.18 的虚拟地址；全列、带网卡名，
    // 回环、链路本地、IPv6 不列，重复的只列一次。
    let cards = [
        ("lo0", v4(127, 0, 0, 1)),
        ("utun4", v4(198, 18, 0, 1)),
        ("en0", v4(172, 20, 10, 12)),
        ("en0", "fe80::1".parse().expect("地址")),
        ("bridge100", v4(169, 254, 1, 1)),
        ("utun5", v4(100, 101, 2, 3)),
        ("en7", v4(172, 20, 10, 12)),
    ]
    .map(|(name, ip)| (name.to_string(), ip));
    let named = |name: &str, ip| (Some(name.to_string()), ip);
    assert_eq!(
        pick(&cards, Some(v4(198, 18, 0, 1))),
        [
            named("utun4", v4(198, 18, 0, 1)),
            named("en0", v4(172, 20, 10, 12)),
            named("utun5", v4(100, 101, 2, 3)),
        ]
    );
}

#[test]
fn no_cards_fall_back_to_the_outward_address() {
    assert_eq!(
        pick(&[], Some(v4(192, 168, 1, 5))),
        [(None, v4(192, 168, 1, 5))]
    );
    assert!(pick(&[], None).is_empty());
    assert!(pick(&[], Some(v4(127, 0, 0, 1))).is_empty(), "回环的不算");
}
