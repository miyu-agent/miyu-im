//! 概率触发（施工 I-197，设计 12 第一节第 1 条）：关着的不抽；开着的照千分比抽，同一条消息每次抽到的一样；别的条件成立的不抽；
//! 没有字的照开关不抽；抽中的交判官、不抬冷静的门槛。

use super::super::test_support::{chatty, facts, hits, now, reply, said, seq};
use super::super::{Judgement, Kind, Route, route, score};
use super::draw;
use crate::Standing;

/// 第 `msg` 条、别的条件都不成立的消息成立的条件。
fn drawn(msg: u64, chatty: &super::super::Chatty) -> Vec<Kind> {
    let mut facts = facts();
    facts.msg = seq(msg);
    hits(&facts, &[], &[], chatty)
        .hits
        .iter()
        .map(|hit| hit.kind)
        .collect()
}

#[test]
fn off_nothing_is_drawn_and_on_about_the_rate_is() {
    let mut chatty = chatty();
    chatty.sample.rate = 1000;
    assert!(drawn(1, &chatty).is_empty(), "关着的不抽");
    chatty.sample.on = true;
    assert_eq!(drawn(1, &chatty), [Kind::Sampled], "千分之一千的每条都中");
    chatty.sample.rate = 0;
    assert!(drawn(1, &chatty).is_empty(), "零是不抽");
    chatty.sample.rate = 50;
    let hit = (1..=2000)
        .filter(|msg| !drawn(*msg, &chatty).is_empty())
        .count();
    assert!((60..=140).contains(&hit), "5% 的两千条里中 {hit} 条");
    assert_eq!(drawn(7, &chatty), drawn(7, &chatty), "同一条每次抽到的一样");
    assert!(draw("qq:1:group:2", 7) < 1000);
}

#[test]
fn a_message_with_another_condition_or_without_words_is_not_drawn() {
    let mut chatty = chatty();
    chatty.sample.on = true;
    chatty.sample.rate = 1000;
    let mut called = facts();
    called.said.addressed = true;
    let got = hits(&called, &[], &[], &chatty);
    assert_eq!(got.primary(), Some(Kind::Direct));
    assert!(!got.sampled(), "冲她来的不算抽中的");
    let mut picture = facts();
    picture.media_only = true;
    assert!(
        hits(&picture, &[], &[], &chatty).hits.is_empty(),
        "没有字的不抽"
    );
    chatty.sample.skip_media = false;
    assert!(
        hits(&picture, &[], &[], &chatty).sampled(),
        "开关关了照样抽"
    );
}

#[test]
fn a_drawn_message_goes_to_the_judge_and_restraint_does_not_lift_it() {
    let mut chatty = chatty();
    chatty.sample.on = true;
    chatty.sample.rate = 1000;
    let plain = facts();
    let got = hits(&plain, &[], &[], &chatty);
    assert_eq!(got.hits[0].bonus, 0.0, "不加分");
    assert_eq!(
        route(&got, &said(Standing::Member), &chatty),
        Route::Judge,
        "判断开着的交判官"
    );
    // 她刚连着回了好几轮：冷静本来要抬门槛，抽中的不抬。
    let busy: Vec<_> = (1..=6).map(|n| reply(n * 20_000, &["qq:10009"])).collect();
    let judgement = Judgement {
        scores: [5.0; 5],
        should_reply: None,
        to_bot: false,
        substance: None,
        severity: None,
        reason: String::new(),
    };
    let sender = plain.said.sender.clone();
    let lifted = score(&judgement, &got, (&busy, &sender), now(), &chatty);
    let calm = score(&judgement, &got, (&[], &sender), now(), &chatty);
    assert_eq!(lifted.threshold, calm.threshold, "冷静不抬抽中的门槛");
}
