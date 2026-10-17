//! 卡片拼出来的规则（施工 O-58 上）：交群聊内核读得出、零问题；每一格落到对的属性；「其他群」不漏进名单里的群；没写的格什么
//! 都不设；字照原样读回；文件名认得出是哪一张卡；读卡片的样子只认这几格。

use serde_json::json;

use miyu_chat::{Rules, Source, Venue, VenueKind};
use miyu_config::Value;

use super::*;

/// 群 `number`。
fn group(number: &str) -> Venue {
    Venue::new("qq", VenueKind::Group, number).expect("合写法")
}

/// 私聊 `number`。
fn private(number: &str) -> Venue {
    Venue::new("qq", VenueKind::Private, number).expect("合写法")
}

/// 照 JSON 读一份卡片。
fn cards(json: serde_json::Value) -> Cards {
    serde_json::from_value(json).expect("卡片的样子")
}

/// 拼好、读出来，断定没有问题；交回套到 `venue` 上每一项的值。
fn values(cards: &Cards, venue: &Venue) -> Vec<(String, Value)> {
    let parsed = Rules::parse(&cards.files());
    assert_eq!(parsed.problems, []);
    parsed
        .rules
        .resolve(venue)
        .entries
        .into_iter()
        .map(|(key, entry)| {
            assert_eq!(entry.origin.source, Source::Settings);
            (key.to_string(), entry.value)
        })
        .collect()
}

/// 字的值。
fn text(value: &str) -> Value {
    Value::Text(value.to_string().into())
}

#[test]
fn without_cards_other_groups_are_off_and_private_chats_untouched() {
    let empty = Cards::default();
    let names: Vec<String> = empty.files().into_iter().map(|file| file.name).collect();
    assert_eq!(names, ["70-card-a.toml"]);
    assert_eq!(
        values(&empty, &group("1")),
        [("allow".to_string(), Value::Bool(false))]
    );
    assert_eq!(values(&empty, &private("1")), []);
}

#[test]
fn every_group_field_lands_on_its_attribute() {
    let full = cards(json!({"groups": [{"id": 7, "set": {
        "on": false,
        "persona": "engineer",
        "extra_prompt": "第一行\n\"第二行\"",
        "keywords": ["miyu", "小羽"],
        "chatty": "many",
        "managers": ["qq:1001"],
        "sleep": "23:00-07:00",
        "rate": "3/60s",
        "show_ids": true,
        "strip_period": false,
        "long_text": "split",
        "long_chars": 500,
    }}]}));
    assert_eq!(
        values(&full, &group("7")),
        [
            ("allow".to_string(), Value::Bool(false)),
            ("chatty.probability".to_string(), Value::Int(120)),
            ("discipline".to_string(), text("chatty")),
            ("extra_prompt".to_string(), text("第一行\n\"第二行\"")),
            (
                "keywords".to_string(),
                Value::List(vec![text("miyu"), text("小羽")])
            ),
            ("managers".to_string(), Value::List(vec![text("qq:1001")])),
            ("outbound.long_chars".to_string(), Value::Int(500)),
            ("outbound.long_text".to_string(), text("split")),
            ("outbound.strip_period".to_string(), Value::Bool(false)),
            ("persona".to_string(), text("engineer")),
            ("rate".to_string(), text("3/60s")),
            ("show_ids".to_string(), Value::Bool(true)),
            ("sleep".to_string(), text("23:00-07:00")),
        ]
    );
}

#[test]
fn chatty_levels_map_to_discipline_and_probability() {
    for (level, discipline, probability) in [
        ("never", "when-called", None),
        ("few", "chatty", Some(20)),
        ("some", "chatty", Some(50)),
        ("many", "chatty", Some(120)),
    ] {
        let one = cards(json!({"groups": [{"id": 1, "set": {"chatty": level}}]}));
        let got = values(&one, &group("1"));
        let find = |key: &str| {
            got.iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.clone())
        };
        assert_eq!(find("discipline"), Some(text(discipline)), "{level}");
        assert_eq!(
            find("chatty.probability"),
            probability.map(Value::Int),
            "{level}"
        );
    }
}

#[test]
fn a_listed_group_does_not_inherit_the_other_groups_card() {
    let set = cards(json!({
        "other_groups": {"on": true, "rate": "1/60s", "persona": "engineer"},
        "groups": [{"id": 7}],
    }));
    assert_eq!(
        values(&set, &group("7")),
        [("allow".to_string(), Value::Bool(true))]
    );
    assert_eq!(
        values(&set, &group("8")),
        [
            ("allow".to_string(), Value::Bool(true)),
            ("persona".to_string(), text("engineer")),
            ("rate".to_string(), text("1/60s")),
        ]
    );
}

#[test]
fn private_cards_set_only_their_own_chat() {
    let set = cards(
        json!({"privates": [{"id": 9, "set": {"persona": "engineer", "strip_period": true, "long_text": "image"}}]}),
    );
    assert_eq!(
        values(&set, &private("9")),
        [
            ("outbound.long_text".to_string(), text("image")),
            ("outbound.strip_period".to_string(), Value::Bool(true)),
            ("persona".to_string(), text("engineer")),
        ]
    );
    assert_eq!(values(&set, &private("10")), []);
    assert_eq!(
        values(&set, &group("9")),
        [("allow".to_string(), Value::Bool(false))]
    );
}

#[test]
fn a_number_written_twice_keeps_the_first_card() {
    let twice = cards(
        json!({"groups": [{"id": 7, "set": {"rate": "1/60s"}}, {"id": 7, "set": {"rate": "2/60s"}}]}),
    );
    assert_eq!(twice.files().len(), 2);
    let rate = values(&twice, &group("7"))
        .into_iter()
        .find(|(key, _)| key == "rate");
    assert_eq!(rate, Some(("rate".to_string(), text("1/60s"))));
}

#[test]
fn file_names_say_which_card() {
    for card in [
        Card::OtherGroups,
        Card::Group(7),
        Card::Private(123_456_789),
    ] {
        assert_eq!(Card::parse(&card.file_name()), Some(card));
    }
    for other in [
        "60-settings.toml",
        "70-card-x-1.toml",
        "70-card-g-.toml",
        "70-card-g-1",
        "80-test.toml",
        "70-card-p-a.toml",
    ] {
        assert_eq!(Card::parse(other), None, "{other}");
    }
    assert_eq!(Card::Group(7).json(), json!({"kind": "group", "id": 7}));
}

#[test]
fn problems_name_the_card() {
    let wrong = cards(
        json!({"groups": [{"id": 7, "set": {"rate": "abc"}}], "privates": [{"id": 9, "set": {"persona": "Bad"}}]}),
    );
    let named: Vec<_> = wrong
        .problems()
        .iter()
        .map(|problem| (Card::parse(&problem.file), problem.key.clone()))
        .collect();
    assert_eq!(
        named,
        [
            (Some(Card::Group(7)), Some("rate".to_string())),
            (Some(Card::Private(9)), Some("persona".to_string()))
        ]
    );
}

#[test]
fn only_known_fields_are_read_and_unset_ones_are_not_written() {
    for bad in [
        json!({"groups": [{"id": 1, "set": {"colour": "red"}}]}),
        json!({"groups": [{"id": -1}]}),
        json!({"privates": [{"id": 1, "set": {"rate": "1/60s"}}]}),
        json!({"groups": [{"id": 1, "set": {"chatty": "always"}}]}),
        json!({"extra": true}),
    ] {
        assert!(
            serde_json::from_value::<Cards>(bad.clone()).is_err(),
            "{bad}"
        );
    }
    let some = cards(json!({"groups": [{"id": 1, "set": {"rate": "1/60s"}}]}));
    assert_eq!(
        serde_json::to_value(&some).expect("写得出"),
        json!({"other_groups": {}, "groups": [{"id": 1, "set": {"rate": "1/60s"}}], "privates": []})
    );
}

#[test]
fn long_text_alone_is_one_outbound_table_and_forward_is_taken() {
    let set = cards(json!({"groups": [{"id": 7, "set": {"long_chars": 0}}]}));
    assert_eq!(
        values(&set, &group("7")),
        [
            ("allow".to_string(), Value::Bool(true)),
            ("outbound.long_chars".to_string(), Value::Int(0)),
        ]
    );
    // 合并转发（施工 O-59 下）。
    let set = cards(json!({"groups": [{"id": 7, "set": {"long_text": "forward"}}]}));
    assert_eq!(
        values(&set, &group("7")),
        [
            ("allow".to_string(), Value::Bool(true)),
            ("outbound.long_text".to_string(), text("forward")),
        ]
    );
    assert!(
        serde_json::from_value::<Cards>(
            json!({"groups": [{"id": 7, "set": {"long_text": "fax"}}]})
        )
        .is_err()
    );
}
