//! 限流（`chat.md` 第二条「守着它的」）：边界（正好 `window` 以前的不算、`now` 那一刻的算、以后的不算）、满了冲她来的、
//! 不冲她来的都只记下、不出声（施工 O-47）、不限、终端管理员和白名单成员；`rate_full`。

use crate::VenueKind;

use super::super::test_support::{PEOPLE, SECOND, at, ctx, judge, member, msg, plus, rate};
use super::super::{Clock, Ctx, Inbound, Outcome, Why, rate_full};

/// 此刻：那一天 UTC 12:00。
fn now() -> Clock {
    at(12, 0)
}

/// 限流 `text`，最近的回合在此刻之前 `ago` 毫秒。
fn limited(text: &str, ago: &[i64]) -> Ctx {
    let mut limited = ctx();
    limited.rate = rate(text);
    limited.turns = ago.iter().map(|ago| plus(now().now, -ago)).collect();
    limited
}

/// 群里别的人冲她来的一条消息。
fn addressed() -> Inbound {
    let mut addressed = member();
    addressed.said.addressed = true;
    addressed
}

fn outcome(msg: &Inbound, ctx: &Ctx) -> Outcome {
    judge(msg, ctx, now()).outcome
}

const LIMITED: Outcome = Outcome::RecordOnly(Why::RateLimited);

#[test]
fn window_is_open_on_the_far_side() {
    // 正好 60 秒以前的不算，差一毫秒的算。
    let edge = limited("3/60s", &[60 * SECOND, SECOND, 0]);
    assert_eq!(outcome(&member(), &edge), Outcome::Pass);
    assert!(!rate_full(&edge, now()));
    let inside = limited("3/60s", &[60 * SECOND - 1, SECOND, 0]);
    assert_eq!(outcome(&member(), &inside), LIMITED);
    assert!(rate_full(&inside, now()));
}

#[test]
fn window_is_closed_on_the_near_side() {
    // 此刻那一回合算，以后的不算。
    assert!(rate_full(&limited("2/60s", &[SECOND, 0]), now()));
    assert!(!rate_full(&limited("2/60s", &[SECOND, -1]), now()));
}

#[test]
fn fewer_turns_than_the_limit_pass() {
    let one_short = limited("3/60s", &[2 * SECOND, SECOND]);
    assert_eq!(outcome(&addressed(), &one_short), Outcome::Pass);
    assert!(!rate_full(&one_short, now()));
}

#[test]
fn turns_may_come_in_any_order() {
    let shuffled = limited("2/60s", &[SECOND, 90 * SECOND, 30 * SECOND]);
    assert!(rate_full(&shuffled, now()));
    assert_eq!(outcome(&addressed(), &shuffled), LIMITED);
}

#[test]
fn not_addressed_is_only_recorded() {
    let full = limited("1/60s", &[0]);
    assert_eq!(outcome(&member(), &full), LIMITED);
    let mut private = msg(crate::Standing::Member, VenueKind::Private);
    private.said.addressed = false;
    assert_eq!(outcome(&private, &full), LIMITED);
}

#[test]
fn full_and_addressed_is_only_recorded_every_time() {
    // 满了冲她来的头一回也只记下，不出声（施工 O-47：QQ 里不发限流提示）；再来、降下去再满都一样。
    let full = limited("2/60s", &[20 * SECOND, 10 * SECOND]);
    assert_eq!(outcome(&addressed(), &full), LIMITED);
    assert_eq!(outcome(&addressed(), &full), LIMITED);
    let again = limited("3/145s", &[150 * SECOND, 140 * SECOND, 130 * SECOND, 0]);
    assert_eq!(outcome(&addressed(), &again), LIMITED);
}

#[test]
fn unlimited_never_fills() {
    let mut open = ctx();
    open.turns = vec![now().now; 50];
    assert_eq!(outcome(&addressed(), &open), Outcome::Pass);
    assert!(!rate_full(&open, now()));
    open.rate = crate::Rate::read("0");
    assert_eq!(outcome(&addressed(), &open), Outcome::Pass);
    assert!(!rate_full(&open, now()));
}

#[test]
fn admin_and_whitelisted_are_not_limited() {
    let full = limited("1/60s", &[0]);
    let expected = [
        Outcome::Pass,
        Outcome::Pass,
        Outcome::Pass,
        LIMITED,
        LIMITED,
    ];
    for ((standing, kind), expected) in PEOPLE.into_iter().zip(expected) {
        let mut sent = msg(standing, kind);
        sent.said.addressed = true;
        assert_eq!(outcome(&sent, &full), expected, "{standing:?} {kind:?}");
    }
}

#[test]
fn rate_full_does_not_look_at_the_sender() {
    // 只看情形和此刻：满了就是满了。
    let full = limited("1/60s", &[0]);
    assert!(rate_full(&full, now()));
    assert!(!rate_full(&ctx(), now()));
}
