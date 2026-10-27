//! 按群冷静（施工 I-62）：数她在这个场所回过的每一轮，不分回的是谁；p 不超过起点不抬，往上照直线抬，到满点抬满；照半衰期衰减；
//! 只管冲她来的；关着的不抬。

use super::super::score::score;
use super::super::test_support::{MINUTE, OTHER, SENDER, chatty, now, person, reply};
use super::super::{
    Chatty, Conditions, Hit, Judgement, Kind, Lift, LiftCtx, PersonRestraint, Reply,
};
use super::Item;

/// 按群冷静开着：10 分钟半衰，起 8、满 20、抬 0.5（10-27 选的档）。
fn on() -> Chatty {
    let mut chatty = chatty();
    chatty.group = PersonRestraint {
        on: true,
        half_life: 10 * MINUTE,
        start: 8.0,
        full: 20.0,
        cap: 0.5,
    };
    chatty
}

/// 只有一种条件 `kind`。
fn only(kind: Kind) -> Conditions {
    Conditions {
        hits: vec![Hit { kind, bonus: 0.15 }],
    }
}

/// `SENDER` 发的一条（条件 `conditions`）门槛抬多少：她最近的回复是 `replies`。
fn lift(conditions: &Conditions, replies: &[Reply], chatty: &Chatty) -> f64 {
    let judgement = Judgement {
        scores: [5.0; 5],
        should_reply: Some(true),
        to_bot: true,
        severity: None,
        reason: String::new(),
    };
    let sender = person(SENDER);
    Item.lift(&LiftCtx {
        judgement: &judgement,
        conditions,
        replies,
        sender: &sender,
        clock: now(),
        chatty,
    })
}

/// 两位小数比。
fn close(got: f64, wanted: f64) -> bool {
    (got - wanted).abs() < 1e-9
}

#[test]
fn every_reply_in_the_venue_counts_whoever_it_was_to() {
    let direct = only(Kind::Direct);
    // 回的全是别人：按人冷静不管，按群照算。8 轮不抬，14 轮抬一半，20 轮以上抬满。
    for (count, wanted) in [(0, 0.0), (8, 0.0), (14, 0.25), (20, 0.5), (30, 0.5)] {
        let replies = vec![reply(0, &[OTHER]); count];
        let got = lift(&direct, &replies, &on());
        assert!(close(got, wanted), "{count} 轮：{got}");
    }
}

#[test]
fn older_replies_count_less_and_later_ones_not_at_all() {
    let direct = only(Kind::Direct);
    // 一个半衰期前的 28 轮算 14：抬一半。
    let replies = vec![reply(10 * MINUTE, &[OTHER]); 28];
    assert!(close(lift(&direct, &replies, &on()), 0.25));
    let later = vec![reply(-MINUTE, &[OTHER]); 30];
    assert!(close(lift(&direct, &later, &on()), 0.0), "晚于此刻的不算");
}

#[test]
fn only_messages_aimed_at_her_and_only_when_on() {
    let replies = vec![reply(0, &[OTHER]); 30];
    assert!(
        close(lift(&only(Kind::AfterSpeaking), &replies, &on()), 0.0),
        "插嘴归冷静管"
    );
    assert!(
        close(lift(&only(Kind::Direct), &replies, &chatty()), 0.0),
        "关着的不抬"
    );
}

#[test]
fn scoring_lifts_the_threshold_by_it() {
    // 挂在门槛修正里（`lifts`）：算分的门槛照它抬。回的全是别人，按人冷静不抬；插嘴的冷静不管冲她来的。
    let judgement = Judgement {
        scores: [5.0; 5],
        should_reply: Some(true),
        to_bot: true,
        severity: None,
        reason: String::new(),
    };
    let replies = vec![reply(0, &[OTHER]); 30];
    let sender = person(SENDER);
    let got = score(
        &judgement,
        &only(Kind::Direct),
        (&replies, &sender),
        now(),
        &on(),
    );
    assert!(close(got.lift, 0.5), "{got:?}");
}
