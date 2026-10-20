//! 群友叫她踢人的二次确认（施工 I-17）：头一次不做；同一轮、同一个人、同样的参数带 `confirm` 才做；换了参数、换了人、下一轮、过了
//! 时的不算。

use serde_json::json;

use super::*;

const LINE: &str = "s-1";

#[test]
fn the_first_call_waits_and_the_same_call_with_confirm_goes() {
    let mut confirms = Confirms::default();
    let now = Instant::now();
    let args = json!({"user": 40001, "reason": "广告"});
    let confirmed = json!({"user": 40001, "reason": "广告", "confirm": true});
    assert!(
        !confirms.check((LINE, 0), Some(40003), &args, now),
        "头一次不做"
    );
    assert!(
        confirms.check((LINE, 1), Some(40003), &confirmed, now),
        "同样的带上 confirm 做"
    );
    assert!(
        !confirms.check((LINE, 2), Some(40003), &confirmed, now),
        "用掉了：再来一次又要确认"
    );
}

#[test]
fn a_confirm_must_match_the_caller_the_args_the_turn_and_the_time() {
    let now = Instant::now();
    let args = json!({"user": 40001});
    let confirmed = json!({"user": 40001, "confirm": true});
    let waiting = || {
        let mut confirms = Confirms::default();
        assert!(!confirms.check((LINE, 0), Some(40003), &args, now));
        confirms
    };
    assert!(
        !waiting().check(
            (LINE, 1),
            Some(40003),
            &json!({"user": 40002, "confirm": true}),
            now
        ),
        "换了人"
    );
    assert!(
        !waiting().check((LINE, 1), Some(40009), &confirmed, now),
        "换了叫她的人"
    );
    assert!(
        !waiting().check(("s-2", 1), Some(40003), &confirmed, now),
        "别的线"
    );
    assert!(
        !waiting().check((LINE, 0), Some(40003), &confirmed, now),
        "下一轮的头一次：上一轮的不算"
    );
    assert!(
        !waiting().check(
            (LINE, 1),
            Some(40003),
            &confirmed,
            now + Duration::from_secs(601)
        ),
        "过了时"
    );
    assert!(
        !Confirms::default().check((LINE, 1), Some(40003), &confirmed, now),
        "没要过确认的直接带 confirm 也不做"
    );
}
