//! 改一张卡：认得三种卡；部分更新只动交来的格、`null` 删掉那一格；认不得的格、类型不对的报在那一格上；规则写不对的报回卡片的格；
//! 不在的卡是 `Missing`；加卡重了照旧，「其他群」删不掉。

use serde_json::{Map, Value, json};

use super::*;

/// 卡片上没写平台的那一个。
fn qq() -> Scope {
    Scope::platform(crate::rules::OLD_PLATFORM)
}

fn cards(value: Value) -> Cards {
    serde_json::from_value(value).expect("是卡片的样子")
}

fn values(value: Value) -> Map<String, Value> {
    value.as_object().expect("是对象").clone()
}

#[test]
fn cards_are_named_by_kind_and_number() {
    assert_eq!(
        card_of(&json!({"kind": "other_groups"}), Some("qq")),
        Some(Card::OtherGroups(qq()))
    );
    assert_eq!(
        card_of(&json!({"kind": "group", "id": 7}), Some("qq")),
        Some(Card::Group(qq(), Id::from(7)))
    );
    assert_eq!(
        card_of(&json!({"kind": "private", "id": 9}), Some("qq")),
        Some(Card::Private(qq(), Id::from(9)))
    );
    assert_eq!(
        card_of(&json!({"kind": "group"}), Some("qq")),
        None,
        "群卡要号"
    );
    assert_eq!(
        card_of(&json!({"kind": "group", "id": "7"}), Some("qq")),
        Some(Card::Group(qq(), Id::from(7))),
        "号写成字的也认（施工 I-79）"
    );
    assert_eq!(
        card_of(&json!({"kind": "group", "id": "a b"}), Some("qq")),
        None
    );
    assert_eq!(card_of(&json!({"kind": "lobby"}), Some("qq")), None);
}

#[test]
fn only_the_given_fields_change_and_null_goes_back_to_default() {
    let before = cards(
        json!({"groups": [{"id": 7, "set": {"rate": "1/60s", "persona": "engineer", "on": false}}]}),
    );
    let got = update(
        &before,
        Card::Group(qq(), Id::from(7)),
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
    let other = update(
        &before,
        Card::OtherGroups(qq()),
        &values(json!({"on": true})),
    );
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
            Card::Group(qq(), Id::from(7)),
            &values(json!({"on": "yes", "nope": 1, "rate": "2/60s"}))
        ),
        Edit::Problems(vec![
            ("nope".into(), "unknown_key".into()),
            ("on".into(), "value".into())
        ])
    );
    assert_eq!(
        update(
            &before,
            Card::Private(qq(), Id::from(9)),
            &values(json!({"rate": "2/60s"}))
        ),
        Edit::Problems(vec![("rate".into(), "unknown_key".into())]),
        "私聊卡没有限流"
    );
    assert_eq!(
        update(
            &before,
            Card::Group(qq(), Id::from(7)),
            &values(json!({"chatty": "loud"}))
        ),
        Edit::Problems(vec![("chatty".into(), "value".into())])
    );
}

#[test]
fn rule_problems_come_back_as_card_fields() {
    let before = cards(json!({"groups": [{"id": 7}]}));
    let Edit::Problems(problems) = update(
        &before,
        Card::Group(qq(), Id::from(7)),
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
    assert_eq!(field_of("outbound.final_only"), "final_only");
    assert_eq!(field_of("sleep"), "sleep");
}

#[test]
fn a_missing_card_is_missing_and_adding_twice_keeps_one() {
    let before = cards(json!({"groups": [{"id": 7, "set": {"rate": "1/60s"}}]}));
    assert_eq!(
        update(
            &before,
            Card::Group(qq(), Id::from(8)),
            &values(json!({"on": true}))
        ),
        Edit::Missing
    );
    assert_eq!(
        update(
            &before,
            Card::Private(qq(), Id::from(9)),
            &values(json!({}))
        ),
        Edit::Missing
    );
    let added = add(&before, Card::Group(qq(), Id::from(7)));
    assert_eq!(added, before, "已经有的照旧，设的格不丢");
    let added = add(
        &add(&before, Card::Group(qq(), Id::from(8))),
        Card::Private(qq(), Id::from(9)),
    );
    assert_eq!(added.groups.len(), 2);
    assert_eq!(added.privates.len(), 1);
    assert_eq!(
        remove(&added, Card::OtherGroups(qq())),
        None,
        "「其他群」删不掉"
    );
    let removed = remove(&added, Card::Group(qq(), Id::from(7))).expect("删得掉");
    assert_eq!(
        removed
            .groups
            .iter()
            .map(|card| card.id.clone())
            .collect::<Vec<_>>(),
        [Id::from(8)]
    );
    assert_eq!(
        remove(&removed, Card::Private(qq(), Id::from(9)))
            .expect("删得掉")
            .privates,
        []
    );
}

#[test]
fn problems_on_other_cards_do_not_block_this_one() {
    // 别的卡本来就写坏了（手改了 `cards.json`）：存这一张只看这一张的问题。
    let mut before = cards(json!({"groups": [{"id": 7}, {"id": 8}]}));
    before.groups[1].set.rate = Some("lots".into());
    let got = update(
        &before,
        Card::Group(qq(), Id::from(7)),
        &values(json!({"rate": "2/60s"})),
    );
    assert!(
        matches!(got, Edit::Done(ref next) if next.groups[0].set.rate.as_deref() == Some("2/60s")),
        "{got:?}"
    );
}

#[test]
fn the_rules_change_key_by_key() {
    // 施工 I-7 中一：卡上的 `rules` 只动交来的键，`null` 去掉那一个，去光了整格去掉。
    let before = cards(
        json!({"groups": [{"id": 7, "set": {"rules": {"chatty.base": 0.7, "parallel": 2}}}]}),
    );
    let changed = values(json!({"rules": {"chatty.base": null, "judge.records": 30}}));
    let Edit::Done(next) = update(&before, Card::Group(qq(), Id::from(7)), &changed) else {
        panic!("改得成");
    };
    assert_eq!(
        serde_json::to_value(&next.groups[0].set).expect("写得成"),
        json!({"rules": {"judge.records": 30, "parallel": 2}})
    );
    let emptied = values(json!({"rules": {"judge.records": null, "parallel": null}}));
    let Edit::Done(last) = update(&next, Card::Group(qq(), Id::from(7)), &emptied) else {
        panic!("改得成");
    };
    assert_eq!(last.groups[0].set.rules, None, "去光了整格去掉");
}

#[test]
fn bad_rule_keys_and_values_are_reported_under_rules() {
    let before = cards(json!({"groups": [{"id": 7, "set": {}}]}));
    let bad_key = values(json!({"rules": {"allow": true, "chatty.base": 0.5}}));
    assert_eq!(
        update(&before, Card::Group(qq(), Id::from(7)), &bad_key),
        Edit::Problems(vec![("rules.allow".to_string(), "unknown_key".to_string())])
    );
    let not_a_map = values(json!({"rules": [1]}));
    assert_eq!(
        update(&before, Card::Group(qq(), Id::from(7)), &not_a_map),
        Edit::Problems(vec![("rules".to_string(), "value".to_string())])
    );
    // 群聊内核查出来的：报在卡上 `rules` 的那个键上。
    let Edit::Problems(problems) = update(
        &before,
        Card::Group(qq(), Id::from(7)),
        &values(json!({"rules": {"judge.records": 0, "chatty.nope": 1}})),
    ) else {
        panic!("规则写错了");
    };
    let keys: Vec<&str> = problems.iter().map(|(key, _)| key.as_str()).collect();
    assert!(keys.contains(&"rules.judge.records"), "{problems:?}");
    assert!(keys.contains(&"rules.chatty.nope"), "{problems:?}");
}

#[test]
fn the_defaults_card_is_edited_like_any_other_and_never_removed() {
    // 施工 I-7 下二：会话默认值那张卡。
    assert_eq!(
        card_of(&json!({"kind": "defaults"}), Some("qq")),
        Some(Card::Defaults)
    );
    let before = cards(json!({}));
    let Edit::Done(next) = update(
        &before,
        Card::Defaults,
        &values(json!({"rate": "2/60s", "rules": {"parallel": 2}})),
    ) else {
        panic!("改得成");
    };
    assert_eq!(
        serde_json::to_value(&next.defaults).expect("写得成"),
        json!({"rate": "2/60s", "rules": {"parallel": 2}})
    );
    assert_eq!(remove(&next, Card::Defaults), None, "删不掉");
    assert_eq!(add(&next, Card::Defaults), *next, "加了照旧");
    let Edit::Problems(problems) = update(&before, Card::Defaults, &values(json!({"rate": "abc"})))
    else {
        panic!("写错了");
    };
    assert_eq!(problems[0].0, "rate", "{problems:?}");
}

#[test]
fn a_card_says_its_platform_or_takes_the_one_given() {
    // 施工 I-80：卡写了平台的照它；没写的照调的一方给的（只挂一个平台时是它），给不出的不认。
    let tg = || Scope::platform("tg");
    assert_eq!(
        card_of(
            &json!({"kind": "group", "platform": "tg", "id": -100}),
            None
        ),
        Some(Card::Group(tg(), Id::from(-100)))
    );
    assert_eq!(
        card_of(&json!({"kind": "group", "id": 7}), Some("tg")),
        Some(Card::Group(tg(), Id::from(7)))
    );
    assert_eq!(card_of(&json!({"kind": "group", "id": 7}), None), None);
    assert_eq!(
        card_of(&json!({"kind": "defaults"}), None),
        Some(Card::Defaults)
    );
    assert_eq!(
        card_of(
            &json!({"kind": "other_groups", "platform": "Bad Name"}),
            Some("qq")
        ),
        None
    );
    // 加、改、删都照平台找：tg 的群 7 不是 QQ 的群 7。
    let both = add(
        &add(&Cards::default(), Card::Group(qq(), Id::from(7))),
        Card::Group(tg(), Id::from(7)),
    );
    assert_eq!(both.groups.len(), 2);
    let values = json!({"persona": "writer"});
    let Edit::Done(changed) = update(
        &both,
        Card::Group(tg(), Id::from(7)),
        values.as_object().expect("对象"),
    ) else {
        panic!("改得了");
    };
    assert_eq!(changed.groups[0].set.persona, None, "QQ 的那张没动");
    assert_eq!(changed.groups[1].set.persona.as_deref(), Some("writer"));
    let left = remove(&changed, Card::Group(qq(), Id::from(7))).expect("删得掉");
    assert_eq!(left.groups.len(), 1);
    assert_eq!(left.groups[0].platform, "tg");
    // tg 的「其他群」改了不碰 QQ 的，改回空的不留空的一格。
    let on = json!({"on": true});
    let Edit::Done(other) = update(
        &Cards::default(),
        Card::OtherGroups(tg()),
        on.as_object().expect("对象"),
    ) else {
        panic!("改得了");
    };
    assert_eq!(other.other_groups, GroupSet::default());
    assert_eq!(other.other_groups_of(&tg()).on, Some(true));
    let off = json!({"on": null});
    let Edit::Done(back) = update(
        &other,
        Card::OtherGroups(tg()),
        off.as_object().expect("对象"),
    ) else {
        panic!("改得了");
    };
    assert!(back.others.is_empty());
}

#[test]
fn a_card_can_be_for_one_bot_and_is_found_by_its_exact_scope() {
    // 施工 I-81 中：卡写了机器人号的只管那个号；加、改、删照平台加号找，整个平台的那张不动。
    let bot = || Scope::of("qq", Some(Id::from(30003)));
    assert_eq!(
        card_of(&json!({"kind": "group", "bot": 30003, "id": 7}), Some("qq")),
        Some(Card::Group(bot(), Id::from(7)))
    );
    assert_eq!(
        card_of(&json!({"kind": "other_groups", "bot": "30003"}), Some("qq")),
        Some(Card::OtherGroups(bot())),
        "号写成字符串也认"
    );
    let both = add(
        &add(&Cards::default(), Card::Group(qq(), Id::from(7))),
        Card::Group(bot(), Id::from(7)),
    );
    assert_eq!(both.groups.len(), 2, "整个平台的一张、这个号的一张");
    let values = json!({"persona": "writer"});
    let Edit::Done(changed) = update(
        &both,
        Card::Group(bot(), Id::from(7)),
        values.as_object().expect("对象"),
    ) else {
        panic!("改得了");
    };
    let mine = changed
        .groups
        .iter()
        .find(|card| card.bot.is_some())
        .expect("有这个号的");
    let whole = changed
        .groups
        .iter()
        .find(|card| card.bot.is_none())
        .expect("有整个平台的");
    assert_eq!(mine.set.persona.as_deref(), Some("writer"));
    assert_eq!(whole.set.persona, None, "整个平台的那张没动");
    let left = remove(&changed, Card::Group(bot(), Id::from(7))).expect("删得掉");
    assert_eq!(left.groups.len(), 1);
    assert_eq!(left.groups[0].bot, None);
}
