//! 按人冷静（`chat.md` 第三条「守着它的」，施工 I-29）：p 不超过起点不抬，往上照直线抬，到满点抬满；只数回他的轮次；照半衰期
//! 衰减；晚于此刻的不算；只管冲她来的；关着的不抬；满点写得不比起点大的一过起点就抬满。

use super::super::test_support::{MINUTE, OTHER, SENDER, chatty, now, person, reply};
use super::super::{Chatty, Conditions, Hit, Judgement, Kind, Lift, LiftCtx, Reply};
use super::{Item, person_pressure};

/// 判官的一般回答。
fn judgement() -> Judgement {
    Judgement {
        scores: [5.0; 5],
        should_reply: Some(true),
        to_bot: true,
        substance: None,
        severity: None,
        reason: String::new(),
    }
}

/// 只有一种条件 `kind`。
fn only(kind: Kind) -> Conditions {
    Conditions {
        hits: vec![Hit { kind, bonus: 0.15 }],
    }
}

/// `SENDER` 冲她来时门槛抬多少：她最近的回复是 `replies`。
fn lift(conditions: &Conditions, replies: &[Reply], chatty: &Chatty) -> f64 {
    let sender = person(SENDER);
    Item.lift(&LiftCtx {
        judgement: &judgement(),
        conditions,
        replies,
        sender: &sender,
        clock: now(),
        chatty,
    })
}

/// 此刻刚回过他 `count` 轮。
fn fresh(count: usize) -> Vec<Reply> {
    vec![reply(0, &[SENDER]); count]
}

/// 两位小数比。
fn close(got: f64, wanted: f64) -> bool {
    (got - wanted).abs() < 1e-9
}

#[test]
fn the_lift_starts_after_two_and_is_full_at_six() {
    let direct = only(Kind::Direct);
    for (count, wanted) in [
        (0, 0.0),
        (1, 0.0),
        (2, 0.0),
        (3, 0.125),
        (4, 0.25),
        (5, 0.375),
        (6, 0.5),
        (9, 0.5),
    ] {
        let got = lift(&direct, &fresh(count), &chatty());
        assert!(
            close(got, wanted),
            "回过 {count} 轮：抬 {got}，该抬 {wanted}"
        );
    }
}

#[test]
fn only_replies_to_him_count_and_they_fade() {
    let replies = [
        vec![reply(0, &[OTHER]); 5],
        vec![reply(0, &[SENDER, OTHER]); 2],
    ]
    .concat();
    assert!(
        close(
            person_pressure(&person(SENDER), &replies, now(), &chatty()),
            2.0
        ),
        "回别人的不算，一轮回几个人的算"
    );
    let faded = vec![reply(10 * MINUTE, &[SENDER]); 4];
    assert!(
        close(
            person_pressure(&person(SENDER), &faded, now(), &chatty()),
            2.0
        ),
        "过一个半衰期算一半"
    );
    let later = vec![reply(-MINUTE, &[SENDER]); 4];
    assert!(
        close(
            person_pressure(&person(SENDER), &later, now(), &chatty()),
            0.0
        ),
        "晚于此刻的不算"
    );
}

#[test]
fn it_only_lifts_calls_and_can_be_turned_off() {
    let busy = fresh(6);
    assert!(
        close(lift(&only(Kind::AfterSpeaking), &busy, &chatty()), 0.0),
        "插嘴归冷静管"
    );
    let mut off = chatty();
    off.person.on = false;
    assert!(
        close(lift(&only(Kind::Direct), &busy, &off), 0.0),
        "关着的不抬"
    );
    let mut flat = chatty();
    flat.person.full = flat.person.start;
    assert!(
        close(lift(&only(Kind::Direct), &fresh(3), &flat), 0.5),
        "满点不比起点大的一过起点就抬满"
    );
}
