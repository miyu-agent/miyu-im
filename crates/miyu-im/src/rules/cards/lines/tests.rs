//! 一张卡拼成的几行（施工 I-7 中一）：卡上的 `rules` 照写在最后、同一个键照它、同一张表并成一行；键写法不对的不写。拼出来交群聊内核
//! 读的整条路在 `cards/tests.rs`、`edit/tests.rs`。

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::super::{Chatty, GroupSet, LongText, PrivateSet};
use super::{group_lines, key_ok, private_lines};

fn rules(pairs: &[(&str, Value)]) -> Option<BTreeMap<String, Value>> {
    Some(
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect(),
    )
}

#[test]
fn keys_are_a_table_and_an_item_or_one_of_the_top_ones() {
    for good in [
        "chatty.base",
        "judge.records",
        "outbound.quote_after",
        "discipline",
        "parallel",
    ] {
        assert!(key_ok(good), "{good}");
    }
    for bad in [
        "allow",
        "match",
        "chatty",
        "chatty.",
        ".base",
        "a.b.c",
        "Chatty.base",
        "chatty.base = 1\nallow",
        "chatty base",
        "",
    ] {
        assert!(!key_ok(bad), "{bad:?}");
    }
}

#[test]
fn the_rules_come_last_and_win_over_the_same_key() {
    let set = GroupSet {
        chatty: Some(Chatty::Few),
        long_text: Some(LongText::Image),
        rules: rules(&[
            ("chatty.probability", json!(70)),
            ("chatty.base", json!(0.75)),
            ("outbound.quote_after", json!(3)),
            ("parallel", json!(2)),
            ("judge.model", json!("deepseek/v4")),
            ("dispatch.supersede_window", json!("7s")),
            ("judge.records", Value::Null),
            ("allow", json!(false)),
        ]),
        ..GroupSet::default()
    };
    assert_eq!(
        group_lines(&set, true),
        [
            "allow = true",
            "discipline = \"chatty\"",
            "chatty = { probability = 70, base = 0.75 }",
            "outbound = { long_text = \"image\", quote_after = 3 }",
            "dispatch = { supersede_window = \"7s\" }",
            "judge = { model = \"deepseek/v4\" }",
            "parallel = 2",
        ]
    );
}

#[test]
fn values_are_written_the_toml_way() {
    let set = PrivateSet {
        rules: rules(&[
            ("chatty.relevance", json!(0.25)),
            ("judge.persona", json!(false)),
            ("outbound.invisible", json!(["\u{200b}"])),
            ("outbound.leak_open", json!({"a\"b": [1, true]})),
        ]),
        ..PrivateSet::default()
    };
    assert_eq!(
        private_lines(&set),
        [
            "chatty = { relevance = 0.25 }",
            "judge = { persona = false }",
            "outbound = { invisible = [\"\u{200b}\"], leak_open = { \"a\\\"b\" = [1, true] } }",
        ]
    );
}

#[test]
fn the_defaults_card_matches_every_venue_and_never_writes_allow() {
    // 施工 I-7 下二：会话默认值那张卡，`allow` 照各自的卡（写了 `on` 也不拼）。
    let set = GroupSet {
        on: Some(false),
        sleep: Some("23:00-07:00".into()),
        rules: rules(&[("chatty.judge_called", json!(false))]),
        ..GroupSet::default()
    };
    assert_eq!(
        super::defaults_lines(&set),
        [
            "sleep = \"23:00-07:00\"",
            "chatty = { judge_called = false }"
        ]
    );
    let cards = super::super::Cards {
        defaults: set,
        ..super::super::Cards::default()
    };
    let file = cards.defaults_file();
    assert_eq!(file.name, "70-card-0.toml", "排在别的卡前面");
    assert!(
        file.text.starts_with("[[rule]]\nmatch = {}\n"),
        "{}",
        file.text
    );
    assert_eq!(cards.files()[0], file);
    assert_eq!(
        super::super::Card::parse("70-card-0.toml"),
        Some(super::super::Card::Defaults)
    );
}
