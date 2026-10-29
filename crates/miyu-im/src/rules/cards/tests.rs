//! 卡片拼出来的规则（施工 O-58 上）：交群聊内核读得出、零问题；每一格落到对的属性；「其他群」不漏进名单里的群；没写的格什么
//! 都不设；字照原样读回；文件名认得出是哪一张卡；读卡片的样子只认这几格；「其他私聊」管陌生人、排开白名单成员和终端管理员。

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

/// 拼好、读出来，断定没有问题；交回套到 `venue` 上每一项的值。私聊卡的号当白名单成员：「其他私聊」不管他们（施工 O-60）。
fn values(cards: &Cards, venue: &Venue) -> Vec<(String, Value)> {
    let listed: Vec<Id> = cards.privates.iter().map(|card| card.id.clone()).collect();
    let parsed = Rules::parse(&cards.files_except(&listed));
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
    assert_eq!(names, ["70-card-a.toml", "70-card-b.toml"]);
    // 「其他群」没写限流的照旧版非白名单群 5 轮/300 秒（施工 I-16）。
    assert_eq!(
        values(&empty, &group("1")),
        [
            ("allow".to_string(), Value::Bool(false)),
            ("rate".to_string(), text("5/300s"))
        ]
    );
    // 「其他私聊」出厂不回复（施工 O-60）。
    assert_eq!(
        values(&empty, &private("1")),
        [("allow".to_string(), Value::Bool(false))]
    );
}

#[test]
fn every_group_field_lands_on_its_attribute() {
    let full = cards(json!({"groups": [{"id": 7, "set": {
        "on": false,
        "persona": "engineer",
        "model": "deepseek/v4",
        "extra_prompt": "第一行\n\"第二行\"",
        "keywords": ["miyu", "小羽"],
        "chatty": "many",
        "managers": ["qq:1001"],
        "sleep": "23:00-07:00",
        "rate": "3/60s",
        "show_ids": true,
        "recent": 30,
        "long_text": "split",
        "long_chars": 500,
        "final_only": true,
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
                Value::List(vec![text("miyu"), text("小羽")].into())
            ),
            (
                "managers".to_string(),
                Value::List(vec![text("qq:1001")].into())
            ),
            ("model".to_string(), text("deepseek/v4")),
            ("outbound.final_only".to_string(), Value::Bool(true)),
            ("outbound.long_chars".to_string(), Value::Int(500)),
            ("outbound.long_text".to_string(), text("split")),
            ("persona".to_string(), text("engineer")),
            ("rate".to_string(), text("3/60s")),
            ("recent".to_string(), Value::Int(30)),
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
        json!({"privates": [{"id": 9, "set": {"persona": "engineer", "long_text": "image", "model": "@fast"}}]}),
    );
    assert_eq!(
        values(&set, &private("9")),
        [
            ("model".to_string(), text("@fast")),
            ("outbound.long_text".to_string(), text("image")),
            ("persona".to_string(), text("engineer")),
        ]
    );
    assert_eq!(
        values(&set, &private("10")),
        [("allow".to_string(), Value::Bool(false))],
        "陌生人照「其他私聊」"
    );
    assert_eq!(
        values(&set, &group("9")),
        [
            ("allow".to_string(), Value::Bool(false)),
            ("rate".to_string(), text("5/300s"))
        ]
    );
}

#[test]
fn a_number_written_twice_keeps_the_first_card() {
    let twice = cards(
        json!({"groups": [{"id": 7, "set": {"rate": "1/60s"}}, {"id": 7, "set": {"rate": "2/60s"}}]}),
    );
    assert_eq!(twice.files().len(), 3);
    let rate = values(&twice, &group("7"))
        .into_iter()
        .find(|(key, _)| key == "rate");
    assert_eq!(rate, Some(("rate".to_string(), text("1/60s"))));
}

#[test]
fn file_names_say_which_card() {
    for card in [
        Card::OtherGroups,
        Card::Group(Id::from(7)),
        Card::Private(Id::from(123_456_789)),
    ] {
        assert_eq!(Card::parse(&card.file_name()), Some(card));
    }
    for other in [
        "60-settings.toml",
        "70-card-x-1.toml",
        "70-card-g-.toml",
        "70-card-g-1",
        "80-test.toml",
        "70-card-p-a%2.toml",
        "70-card-p-a%3Ab.toml",
    ] {
        assert_eq!(Card::parse(other), None, "{other}");
    }
    assert_eq!(
        Card::Group(Id::from(7)).json(),
        json!({"kind": "group", "id": 7})
    );
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
            (Some(Card::Group(Id::from(7))), Some("rate".to_string())),
            (
                Some(Card::Private(Id::from(9))),
                Some("persona".to_string())
            )
        ]
    );
}

#[test]
fn only_known_fields_are_read_and_unset_ones_are_not_written() {
    for bad in [
        json!({"groups": [{"id": 1, "set": {"colour": "red"}}]}),
        json!({"groups": [{"id": "a b"}]}),
        json!({"groups": [{"id": 1.5}]}),
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
        json!({"other_groups": {}, "groups": [{"id": 1, "set": {"rate": "1/60s"}}], "privates": [], "other_privates": {}})
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

#[test]
fn the_final_period_is_no_longer_a_card_field() {
    // 去句号是通用的配置项（施工 O-58 补二）：卡片不收。
    assert!(
        serde_json::from_value::<Cards>(
            json!({"groups": [{"id": 7, "set": {"strip_period": false}}]})
        )
        .is_err()
    );
}

#[test]
fn other_private_chats_have_their_own_card_that_leaves_known_people_alone() {
    // 「其他私聊」（施工 O-60）：回复、人格、附加提示词、限流、出站的几格；白名单成员和认出来的终端管理员排开。
    let set = cards(json!({"other_privates": {
        "on": true,
        "persona": "engineer",
        "model": "deepseek/v4",
        "extra_prompt": "陌生人",
        "rate": "3/60s",
        "final_only": true,
        "long_text": "split",
    }}));
    let parsed = Rules::parse(&set.files_except(&[Id::from(20), Id::from(30)]));
    assert_eq!(parsed.problems, []);
    let strangers: Vec<String> = parsed
        .rules
        .resolve(&private("10"))
        .entries
        .into_keys()
        .map(|key| key.to_string())
        .collect();
    assert_eq!(
        strangers,
        [
            "allow",
            "extra_prompt",
            "model",
            "outbound.final_only",
            "outbound.long_text",
            "persona",
            "rate"
        ]
    );
    assert!(
        parsed.rules.resolve(&private("20")).entries.is_empty(),
        "白名单成员不套"
    );
    assert!(
        parsed.rules.resolve(&private("30")).entries.is_empty(),
        "终端管理员不套"
    );
    assert_eq!(Card::parse("70-card-b.toml"), Some(Card::OtherPrivates));
    assert_eq!(
        Card::OtherPrivates.json(),
        json!({"kind": "other_privates"})
    );
    assert!(
        serde_json::from_value::<Cards>(json!({"other_privates": {"keywords": ["x"]}})).is_err(),
        "触发词是群的"
    );
}

#[test]
fn ids_written_as_text_and_negative_numbers_make_cards_too() {
    // 施工 I-79：别的平台的号是一串字（可能带 `/`、`@`），Telegram 的群号是负数。文件名照写法转义、认得回来；规则里字照字写。
    let mixed = cards(json!({
        "groups": [
            {"id": -1_001_987_654_321_i64, "set": {"persona": "engineer"}},
            {"id": "room/1@chatroom", "set": {"persona": "engineer"}},
        ],
        "privates": [{"id": "wxid_abc", "set": {"persona": "engineer"}}],
    }));
    let names: Vec<String> = mixed.files().into_iter().map(|file| file.name).collect();
    assert_eq!(
        names,
        [
            "70-card-a.toml",
            "70-card-b.toml",
            "70-card-g--1001987654321.toml",
            "70-card-g-room%2F1@chatroom.toml",
            "70-card-p-wxid_abc.toml",
        ]
    );
    for name in &names[2..] {
        assert!(Card::parse(name).is_some(), "{name} 认得回来");
    }
    assert_eq!(
        Card::parse("70-card-g-room%2F1@chatroom.toml"),
        Some(Card::Group(Id::new("room/1@chatroom").expect("合写法")))
    );
    assert_eq!(Card::parse("70-card-g-room%2.toml"), None, "转义不全的不认");
    let engineer = ("persona".to_string(), text("engineer"));
    for venue in [
        group("-1001987654321"),
        group("room/1@chatroom"),
        private("wxid_abc"),
    ] {
        assert!(values(&mixed, &venue).contains(&engineer), "{venue:?}");
    }
    // 别的群、别的私聊不受这几张卡影响。
    assert!(!values(&mixed, &group("1")).contains(&engineer));
    assert!(!values(&mixed, &private("wxid_abd")).contains(&engineer));
}

#[test]
fn qq_numbers_are_saved_as_numbers() {
    // 只用 QQ 的人存下来的 `cards.json` 不变（施工 I-79）。
    let saved = serde_json::to_value(cards(
        json!({"groups": [{"id": 130_515_298}], "privates": [{"id": 10001}]}),
    ))
    .expect("写得出");
    assert_eq!(saved["groups"][0]["id"], json!(130_515_298));
    assert_eq!(saved["privates"][0]["id"], json!(10001));
    // 拼出来的规则里 QQ 的号照旧写成数。
    let qq = cards(json!({"groups": [{"id": 130_515_298, "set": {"persona": "engineer"}}]}));
    let file = qq
        .files()
        .into_iter()
        .find(|file| file.name == "70-card-g-130515298.toml")
        .expect("有这个群的卡");
    assert!(file.text.contains("group = [130515298]"), "{}", file.text);
}
