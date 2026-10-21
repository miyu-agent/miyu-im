//! 跟进（施工 I-21）：她正在为他跑一轮才算；开关关了、@ 了别人、引用别人的不算；只有图的照算；和冲她来同时成立的加分相加、
//! 主触发是冲她来；没冲她来的主触发是跟进、排在续聊后面。

use super::super::test_support::{OTHER, SENDER, chatty, facts, hits_serving, mills, person};
use super::super::{Conditions, Kind};

fn kinds(conditions: &Conditions) -> Vec<Kind> {
    conditions.hits.iter().map(|hit| hit.kind).collect()
}

#[test]
fn it_holds_while_she_is_answering_him() {
    let got = hits_serving(&facts(), &[person(SENDER)], &chatty());
    assert_eq!(kinds(&got), [Kind::FollowUp]);
    assert_eq!(mills(got.hits[0].bonus), 100, "加分照参数");
    assert_eq!(got.primary(), Some(Kind::FollowUp));
    let other = hits_serving(&facts(), &[person(OTHER)], &chatty());
    assert!(other.hits.is_empty(), "她在回别人：不算");
    assert!(hits_serving(&facts(), &[], &chatty()).hits.is_empty());
}

#[test]
fn the_switch_turns_it_off() {
    let mut params = chatty();
    params.followup.on = false;
    assert!(
        hits_serving(&facts(), &[person(SENDER)], &params)
            .hits
            .is_empty()
    );
}

#[test]
fn words_for_someone_else_are_not_a_followup() {
    let mut mentioning = facts();
    mentioning.mentions_others = true;
    assert!(
        hits_serving(&mentioning, &[person(SENDER)], &chatty())
            .hits
            .is_empty()
    );
    let mut quoting = facts();
    quoting.quotes_other = true;
    assert!(
        hits_serving(&quoting, &[person(SENDER)], &chatty())
            .hits
            .is_empty()
    );
}

#[test]
fn pictures_count_and_direct_comes_first() {
    let mut picture = facts();
    picture.media_only = true;
    picture.textless = true;
    let got = hits_serving(&picture, &[person(SENDER)], &chatty());
    assert_eq!(kinds(&got), [Kind::FollowUp], "只有图的照算（同续聊）");
    let mut called = facts();
    called.said.addressed = true;
    let got = hits_serving(&called, &[person(SENDER)], &chatty());
    assert_eq!(kinds(&got), [Kind::Direct, Kind::FollowUp]);
    assert_eq!(got.primary(), Some(Kind::Direct));
}
