//! 令牌猜错的限次（施工 I-117）。

use std::net::{IpAddr, Ipv4Addr};

use super::*;

fn ip(last: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, last))
}

#[test]
fn enough_misses_block_that_source_for_the_rest_of_the_window() {
    let guesses = Guesses::default();
    let start = Instant::now();
    assert_eq!(guesses.missed(ip(1), start), Noted::First);
    for _ in 2..TRIES {
        assert_eq!(guesses.missed(ip(1), start), Noted::Quiet, "中间的不记");
    }
    assert!(!guesses.blocked(ip(1), start));
    assert_eq!(guesses.missed(ip(1), start), Noted::Blocked(TRIES));
    assert!(guesses.blocked(ip(1), start));
    assert!(!guesses.blocked(ip(2), start), "别的来源不受影响");
    let later = start + WINDOW;
    assert!(!guesses.blocked(ip(1), later), "过了一段放开");
    assert_eq!(guesses.missed(ip(1), later), Noted::First, "从头数");
}
