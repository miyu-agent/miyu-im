//! 配置项那一层拼出来的规则（施工 O-41）：交群聊内核读得出、零问题；七项各设到哪种场所；没设、默认的什么都不设；字照原样
//! 读回；写错的只丢那一项、来处说得回配置项的键；推来的变化。

use serde_json::{Value as Json, json};

use miyu_chat::{Rules, Source, Venue, VenueKind};
use miyu_config::Value;
use miyu_config::problem::Code;

use super::*;

/// 群 `number`。
fn group(number: &str) -> Venue {
    Venue::new("qq", VenueKind::Group, number).expect("合写法")
}

/// 私聊 `number`。
fn private(number: &str) -> Venue {
    Venue::new("qq", VenueKind::Private, number).expect("合写法")
}

/// 照 `config` 拼一份、读出来，断定没有问题。
fn rules(config: Json) -> Rules {
    let parsed = Rules::parse(&[Configured::handed(&config).file()]);
    assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
    parsed.rules
}

/// 套到 `venue` 上每一项的值和第几条规则，照键排；来处都该是配置项那一份。
fn values(rules: &Rules, venue: &Venue) -> Vec<(String, Value, usize)> {
    rules
        .resolve(venue)
        .entries
        .iter()
        .map(|(key, entry)| {
            assert_eq!(entry.origin.source, Source::Settings);
            assert_eq!(entry.origin.file, FILE);
            (key.to_string(), entry.value.clone(), entry.origin.rule)
        })
        .collect()
}

/// 字的值。
fn text(value: &str) -> Value {
    Value::Text(value.to_string().into())
}

/// 七项都设了的一份。
fn everything() -> Json {
    json!({
        "onebot.groups": [10, 11],
        "onebot.other_groups": false,
        "onebot.group_rate": "30/60s",
        "onebot.other_group_rate": "1/600s",
        "onebot.sleep": "23:00-07:00",
        "onebot.keywords": ["miyu", "小羽"],
        "onebot.show_ids": true,
    })
}

#[test]
fn nothing_set_or_defaults_set_nothing() {
    let defaults = json!({"onebot.other_groups": true, "onebot.show_ids": false, "onebot.groups": [], "onebot.keywords": []});
    for config in [json!({}), json!(null), defaults] {
        let rules = rules(config);
        for venue in [group("10"), private("10")] {
            assert_eq!(values(&rules, &venue), []);
        }
    }
}

#[test]
fn each_item_lands_on_its_venues() {
    let rules = rules(everything());
    let sleep = ("sleep".to_string(), text("23:00-07:00"), 4);
    let show_ids = ("show_ids".to_string(), Value::Bool(true), 5);
    let keywords = (
        "keywords".to_string(),
        Value::List(vec![text("miyu"), text("小羽")]),
        3,
    );
    // 白名单群：白名单那两条盖掉对全部群的限流、不接话。
    assert_eq!(
        values(&rules, &group("11")),
        [
            ("allow".to_string(), Value::Bool(true), 6),
            keywords.clone(),
            ("rate".to_string(), text("30/60s"), 7),
            show_ids.clone(),
            sleep.clone(),
        ]
    );
    assert_eq!(
        values(&rules, &group("12")),
        [
            ("allow".to_string(), Value::Bool(false), 2),
            keywords,
            ("rate".to_string(), text("1/600s"), 1),
            show_ids.clone(),
            sleep.clone(),
        ]
    );
    // 私聊只有睡觉时间、看得到 QQ 号。
    assert_eq!(values(&rules, &private("10")), [show_ids, sleep]);
}

#[test]
fn an_empty_whitelist_gives_the_whitelist_rate_nowhere() {
    let rules = rules(json!({"onebot.group_rate": "30/60s", "onebot.other_groups": false}));
    assert_eq!(
        values(&rules, &group("10")),
        [("allow".to_string(), Value::Bool(false), 2)]
    );
}

#[test]
fn awkward_words_come_back_as_written() {
    let words = ["a\"b", "c\\d", "'g'", "h = 1", "]]", "#x", "小羽"];
    let rules = rules(json!({ "onebot.keywords": words }));
    let got = values(&rules, &group("1"));
    assert_eq!(
        got,
        [(
            "keywords".to_string(),
            Value::List(words.iter().map(|word| text(word)).collect()),
            3
        )]
    );
    // 带控制字符的（换行也算）照规则的规矩整项不收，说是哪一项的。
    for bad in ["e\nf", "\u{1}"] {
        let config = json!({ "onebot.keywords": ["ok", bad] });
        let parsed = Rules::parse(&[Configured::handed(&config).file()]);
        let named: Vec<_> = parsed
            .problems
            .iter()
            .map(|problem| (problem.code, problem.rule.and_then(key_of)))
            .collect();
        assert_eq!(
            named,
            [(Code::BadFormat, Some("onebot.keywords".to_string()))],
            "{bad:?}"
        );
    }
}

#[test]
fn a_bad_value_drops_only_that_item_and_names_the_setting() {
    let mut config = everything();
    config["onebot.group_rate"] = json!("abc");
    let parsed = Rules::parse(&[Configured::handed(&config).file()]);
    let seen: Vec<_> = parsed
        .problems
        .iter()
        .map(|problem| {
            (
                problem.code,
                problem.source,
                problem.rule.and_then(key_of),
                problem.key.clone(),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [(
            Code::BadFormat,
            Source::Settings,
            Some("onebot.group_rate".to_string()),
            Some("rate".to_string())
        )]
    );
    // 白名单群的限流照其他群的，别的照用。
    let entries = parsed.rules.resolve(&group("10")).entries;
    assert_eq!(entries["rate"].value, text("1/600s"));
    assert_eq!(entries["allow"].value, Value::Bool(true));
}

#[test]
fn rules_name_their_settings_in_order() {
    let keys: Vec<_> = (0..=8).map(key_of).collect();
    assert_eq!(
        keys,
        [
            None,
            Some("onebot.other_group_rate".to_string()),
            Some("onebot.other_groups".to_string()),
            Some("onebot.keywords".to_string()),
            Some("onebot.sleep".to_string()),
            Some("onebot.show_ids".to_string()),
            Some("onebot.groups".to_string()),
            Some("onebot.group_rate".to_string()),
            None,
        ]
    );
}

#[test]
fn pushed_changes_merge_and_say_whether_anything_moved() {
    let mut configured = Configured::handed(&everything());
    let unchanged =
        json!({"onebot.listen": 9000, "onebot.sleep": "23:00-07:00", "other.sleep": "off"});
    assert!(!configured.change(unchanged.as_object().expect("对象")));
    // `null` 回到默认：白名单以外的群又接话，限流照规则文件。
    let reset =
        json!({"onebot.other_groups": null, "onebot.group_rate": null, "onebot.groups": null});
    assert!(configured.change(reset.as_object().expect("对象")));
    let rules = Rules::parse(&[configured.file()]).rules;
    let entries = rules.resolve(&group("10")).entries;
    assert!(!entries.contains_key("allow"));
    assert_eq!(entries["rate"].value, text("1/600s"));
    // 空的字当没设：不拼进去，也不报写错。
    assert!(configured.change(json!({"onebot.sleep": ""}).as_object().expect("对象")));
    let parsed = Rules::parse(&[configured.file()]);
    assert_eq!(parsed.problems, []);
    assert!(
        !parsed
            .rules
            .resolve(&private("1"))
            .entries
            .contains_key("sleep")
    );
}

#[test]
fn misread_names_each_bad_setting_once_in_order() {
    // 触发词里坏了两个也只报一处。
    assert_eq!(
        Configured::handed(&everything()).misread(),
        Vec::<String>::new()
    );
    let mut config = everything();
    config["onebot.group_rate"] = json!("abc");
    config["onebot.sleep"] = json!("25:00-26:00");
    config["onebot.keywords"] = json!(["a\nb", "c\u{1}"]);
    config["onebot.other_group_rate"] = json!("5/0s");
    assert_eq!(
        Configured::handed(&config).misread(),
        [
            "onebot.other_group_rate",
            "onebot.keywords",
            "onebot.sleep",
            "onebot.group_rate"
        ]
    );
}
