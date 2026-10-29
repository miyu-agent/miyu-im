//! 号是字、负数的卡（施工 I-79）；卡片带平台：旧卡算 QQ、卡只管自己平台、挂着的平台各有兜底卡、白名单照平台排、平台名要合写法（施工 I-80）。

use super::*;

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
        Some(Card::Group(
            qq(),
            Id::new("room/1@chatroom").expect("合写法")
        ))
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

/// 平台 `platform` 上种类是 `kind`、号是 `number` 的场所。
fn on(platform: &str, kind: VenueKind, number: &str) -> Venue {
    Venue::new(platform, kind, number).expect("合写法")
}

/// 照挂着的平台 `mounted`、白名单 `known` 拼好读出来，断定没有问题；交回套到 `venue` 上每一项的值。
fn values_on(
    cards: &Cards,
    (mounted, known): (&[&str], &[String]),
    venue: &Venue,
) -> Vec<(String, Value)> {
    let parsed = Rules::parse(&cards.files_for(mounted, known));
    assert_eq!(parsed.problems, []);
    parsed
        .rules
        .resolve(venue)
        .entries
        .into_iter()
        .map(|(key, entry)| (key.to_string(), entry.value))
        .collect()
}

#[test]
fn old_cards_are_qq_cards_and_are_saved_byte_for_byte() {
    // 施工 I-80：I-80 以前的 `cards.json` 没有平台，算 QQ 的；原样存回去一个字节不变。
    let text = r#"{"other_groups":{"on":true},"groups":[{"id":123,"set":{"persona":"engineer"}}],"privates":[{"id":5,"set":{}}],"other_privates":{}}"#;
    let old: Cards = serde_json::from_str(text).expect("读得出");
    assert_eq!(old.groups[0].platform, OLD_PLATFORM);
    assert_eq!(old.privates[0].platform, OLD_PLATFORM);
    assert_eq!(serde_json::to_string(&old).expect("写得出"), text);
}

#[test]
fn a_card_only_holds_on_its_own_platform() {
    let mixed = cards(json!({
        "groups": [
            {"id": 123, "set": {"persona": "engineer"}},
            {"platform": "tg", "id": -100, "set": {"persona": "writer"}},
        ],
    }));
    let names: Vec<String> = mixed
        .files_for(&["qq", "tg"], &[])
        .into_iter()
        .map(|file| file.name)
        .collect();
    assert_eq!(
        names,
        [
            "70-card-a.toml",
            "70-card-b.toml",
            "70-card-tg~a.toml",
            "70-card-tg~b.toml",
            "70-card-g-123.toml",
            "70-card-tg~g--100.toml",
        ]
    );
    for name in &names {
        assert!(Card::parse(name).is_some(), "{name} 认得回来");
    }
    assert_eq!(
        Card::parse("70-card-tg~g--100.toml"),
        Some(Card::Group("tg".to_string(), Id::from(-100)))
    );
    let mounted: &[&str] = &["qq", "tg"];
    let engineer = ("persona".to_string(), text("engineer"));
    let writer = ("persona".to_string(), text("writer"));
    assert!(
        values_on(&mixed, (mounted, &[]), &on("qq", VenueKind::Group, "123")).contains(&engineer)
    );
    assert!(
        !values_on(&mixed, (mounted, &[]), &on("tg", VenueKind::Group, "123")).contains(&engineer),
        "QQ 群 123 的卡管不到 tg 群 123"
    );
    assert!(
        values_on(&mixed, (mounted, &[]), &on("tg", VenueKind::Group, "123"))
            .contains(&("allow".to_string(), Value::Bool(false))),
        "tg 群 123 不在 tg 的名单里：照 tg 的「其他群」不回复"
    );
    assert!(
        values_on(&mixed, (mounted, &[]), &on("tg", VenueKind::Group, "-100")).contains(&writer)
    );
}

#[test]
fn every_mounted_platform_gets_its_own_fallback_cards() {
    // 施工 I-80：挂着的平台没写过「其他群」「其他私聊」的照空卡，陌生的群、私聊照旧不回复；QQ 的卡不漏过去。
    let mut set = cards(json!({"other_groups": {"on": true}, "other_privates": {"on": true}}));
    let mounted: &[&str] = &["qq", "tg"];
    let allow = |on: bool| ("allow".to_string(), Value::Bool(on));
    assert!(
        values_on(&set, (mounted, &[]), &on("qq", VenueKind::Group, "1")).contains(&allow(true))
    );
    assert!(
        values_on(&set, (mounted, &[]), &on("tg", VenueKind::Group, "1")).contains(&allow(false))
    );
    assert!(
        values_on(&set, (mounted, &[]), &on("tg", VenueKind::Private, "1")).contains(&allow(false))
    );
    set.other_groups_mut("tg").on = Some(true);
    assert!(
        values_on(&set, (mounted, &[]), &on("tg", VenueKind::Group, "1")).contains(&allow(true))
    );
    assert_eq!(set.others.len(), 1, "tg 的写进 others");
    let saved = serde_json::to_value(&set).expect("写得出");
    assert_eq!(saved["others"], json!({"tg": {"groups": {"on": true}}}));
}

#[test]
fn whitelist_members_are_spared_only_on_their_own_platform() {
    let set = cards(
        json!({"other_privates": {"on": true}, "others": {"tg": {"privates": {"on": true}}}}),
    );
    let mounted: &[&str] = &["qq", "tg"];
    let known = ["qq:5".to_string()];
    let allowed = |venue: &Venue| {
        values_on(&set, (mounted, &known), venue)
            .contains(&("allow".to_string(), Value::Bool(true)))
    };
    assert!(
        !allowed(&on("qq", VenueKind::Private, "5")),
        "QQ 的白名单成员不归「其他私聊」管"
    );
    assert!(allowed(&on("qq", VenueKind::Private, "6")));
    assert!(
        allowed(&on("tg", VenueKind::Private, "5")),
        "tg 上的 5 是陌生人：照 tg 的「其他私聊」（开着）"
    );
}

#[test]
fn platform_names_are_checked() {
    let bad = cards(json!({
        "groups": [{"platform": "Bad Name", "id": 1}],
        "others": {"qq": {}, "tg": {}},
    }));
    assert_eq!(bad.bad_platforms(), ["Bad Name", "qq"]);
    assert_eq!(
        cards(json!({"others": {"tg": {}}})).bad_platforms(),
        Vec::<String>::new()
    );
    for other in [
        "70-card-qq~a.toml",
        "70-card-~a.toml",
        "70-card-T G~a.toml",
        "70-card-tg~0.toml",
    ] {
        assert_eq!(Card::parse(other), None, "{other}");
    }
}
