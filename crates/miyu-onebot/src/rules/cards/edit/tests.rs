//! 改一张卡：认得三种卡；部分更新只动交来的格、`null` 删掉那一格；认不得的格、类型不对的报在那一格上；规则写不对的报回卡片的格；
//! 不在的卡是 `Missing`；加卡重了照旧，「其他群」删不掉。

use serde_json::{Map, Value, json};

use super::*;

fn cards(value: Value) -> Cards {
    serde_json::from_value(value).expect("是卡片的样子")
}

fn values(value: Value) -> Map<String, Value> {
    value.as_object().expect("是对象").clone()
}

#[test]
fn cards_are_named_by_kind_and_number() {
    assert_eq!(
        card_of(&json!({"kind": "other_groups"})),
        Some(Card::OtherGroups)
    );
    assert_eq!(
        card_of(&json!({"kind": "group", "id": 7})),
        Some(Card::Group(7))
    );
    assert_eq!(
        card_of(&json!({"kind": "private", "id": 9})),
        Some(Card::Private(9))
    );
    assert_eq!(card_of(&json!({"kind": "group"})), None, "群卡要号");
    assert_eq!(card_of(&json!({"kind": "group", "id": "7"})), None);
    assert_eq!(card_of(&json!({"kind": "lobby"})), None);
}

#[test]
fn only_the_given_fields_change_and_null_goes_back_to_default() {
    let before = cards(
        json!({"groups": [{"id": 7, "set": {"rate": "1/60s", "persona": "engineer", "on": false}}]}),
    );
    let got = update(
        &before,
        Card::Group(7),
        &values(json!({"rate": "2/60s", "persona": null, "long_chars": 500})),
    );
    let Edit::Done(after) = got else {
        panic!("该存：{got:?}");
    };
    let set = &after.groups[0].set;
    assert_eq!(set.rate.as_deref(), Some("2/60s"));
    assert_eq!(set.persona, None, "null 是恢复默认");
    assert_eq!(set.on, Some(false), "没交的不动");
    assert_eq!(set.long_chars, Some(500));
    let other = update(&before, Card::OtherGroups, &values(json!({"on": true})));
    assert!(
        matches!(other, Edit::Done(ref next) if next.other_groups.on == Some(true) && next.groups == before.groups)
    );
}

#[test]
fn bad_fields_are_reported_on_themselves_and_nothing_changes() {
    let before = cards(json!({"groups": [{"id": 7}], "privates": [{"id": 9}]}));
    assert_eq!(
        update(
            &before,
            Card::Group(7),
            &values(json!({"on": "yes", "nope": 1, "rate": "2/60s"}))
        ),
        Edit::Problems(vec![
            ("nope".into(), "unknown_key".into()),
            ("on".into(), "value".into())
        ])
    );
    assert_eq!(
        update(&before, Card::Private(9), &values(json!({"rate": "2/60s"}))),
        Edit::Problems(vec![("rate".into(), "unknown_key".into())]),
        "私聊卡没有限流"
    );
    assert_eq!(
        update(&before, Card::Group(7), &values(json!({"chatty": "loud"}))),
        Edit::Problems(vec![("chatty".into(), "value".into())])
    );
}

#[test]
fn rule_problems_come_back_as_card_fields() {
    let before = cards(json!({"groups": [{"id": 7}]}));
    let Edit::Problems(problems) = update(
        &before,
        Card::Group(7),
        &values(json!({"rate": "lots", "long_chars": 200_000})),
    ) else {
        panic!("该拒");
    };
    let keys: Vec<&str> = problems.iter().map(|(key, _)| key.as_str()).collect();
    assert!(keys.contains(&"rate"), "{problems:?}");
    assert!(
        keys.contains(&"long_chars"),
        "出站表里的去掉 outbound.：{problems:?}"
    );
    assert_eq!(field_of("allow"), "on");
    assert_eq!(field_of("chatty.probability"), "chatty");
    assert_eq!(field_of("discipline"), "chatty");
    assert_eq!(field_of("outbound.strip_period"), "strip_period");
    assert_eq!(field_of("sleep"), "sleep");
}

#[test]
fn a_missing_card_is_missing_and_adding_twice_keeps_one() {
    let before = cards(json!({"groups": [{"id": 7, "set": {"rate": "1/60s"}}]}));
    assert_eq!(
        update(&before, Card::Group(8), &values(json!({"on": true}))),
        Edit::Missing
    );
    assert_eq!(
        update(&before, Card::Private(9), &values(json!({}))),
        Edit::Missing
    );
    let added = add(&before, Card::Group(7));
    assert_eq!(added, before, "已经有的照旧，设的格不丢");
    let added = add(&add(&before, Card::Group(8)), Card::Private(9));
    assert_eq!(added.groups.len(), 2);
    assert_eq!(added.privates.len(), 1);
    assert_eq!(remove(&added, Card::OtherGroups), None, "「其他群」删不掉");
    let removed = remove(&added, Card::Group(7)).expect("删得掉");
    assert_eq!(
        removed
            .groups
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        [8]
    );
    assert_eq!(
        remove(&removed, Card::Private(9)).expect("删得掉").privates,
        []
    );
}

#[test]
fn problems_on_other_cards_do_not_block_this_one() {
    // 别的卡本来就写坏了（手改了 `cards.json`）：存这一张只看这一张的问题。
    let mut before = cards(json!({"groups": [{"id": 7}, {"id": 8}]}));
    before.groups[1].set.rate = Some("lots".into());
    let got = update(&before, Card::Group(7), &values(json!({"rate": "2/60s"})));
    assert!(
        matches!(got, Edit::Done(ref next) if next.groups[0].set.rate.as_deref() == Some("2/60s")),
        "{got:?}"
    );
}
