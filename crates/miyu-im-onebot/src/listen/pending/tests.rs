//! 名额按来源分（施工 I-107）。

use std::net::{IpAddr, Ipv4Addr};

use super::*;

fn ip(last: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, last))
}

#[test]
fn one_source_cannot_take_everyone_elses_slots() {
    let pending = Pending::new(4, 2);
    let held: Vec<Slot> = (0..2)
        .map(|_| pending.take(ip(1)).expect("有名额"))
        .collect();
    assert_eq!(
        pending.take(ip(1)).err(),
        Some(Full::Source),
        "这个来源满了"
    );
    let other = pending.take(ip(2)).expect("别的来源照样有");
    drop(held);
    assert!(pending.take(ip(1)).is_ok(), "放下了就让出来");
    drop(other);
}

#[test]
fn the_total_still_caps_every_source_together() {
    let pending = Pending::new(3, 2);
    let _held: Vec<Slot> = [1, 2, 3]
        .iter()
        .map(|&last| pending.take(ip(last)).expect("有名额"))
        .collect();
    assert_eq!(pending.take(ip(4)).err(), Some(Full::Total));
}
