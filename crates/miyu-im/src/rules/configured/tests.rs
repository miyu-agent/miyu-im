//! 配置项那一层拼出来的规则（施工 O-41）：交群聊内核读得出、零问题；三项各设到哪种场所（O-58 上：群相关四项挪进卡片）；没设、默认的什么都不设；字照原样
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

/// 三项都设了的一份；去掉的四项（O-58 上挪进卡片）写着也不认。
fn everything() -> Json {
    json!({
        "im.groups": [10, 11],
        "im.other_groups": false,
        "im.group_rate": "30/60s",
        "im.sleep": "23:00-07:00",
        "im.keywords": ["miyu", "小羽"],
        "im.show_ids": true,
    })
}

#[test]
fn nothing_set_or_defaults_set_nothing() {
    let defaults = json!({"im.show_ids": false, "im.keywords": [], "im.strip_period": true});
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
    let keywords = (
        "keywords".to_string(),
        Value::List(vec![text("miyu"), text("小羽")]),
        1,
    );
    let sleep = ("sleep".to_string(), text("23:00-07:00"), 2);
    let show_ids = ("show_ids".to_string(), Value::Bool(true), 3);
    // 群：三项都在，没有接不接话、限流（挪进卡片了）。
    for number in ["11", "12"] {
        assert_eq!(
            values(&rules, &group(number)),
            [keywords.clone(), show_ids.clone(), sleep.clone()]
        );
    }
    // 私聊只有睡觉时间、看得到 QQ 号。
    assert_eq!(values(&rules, &private("10")), [show_ids, sleep]);
}

#[test]
fn awkward_words_come_back_as_written() {
    let words = ["a\"b", "c\\d", "'g'", "h = 1", "]]", "#x", "小羽"];
    let rules = rules(json!({ "im.keywords": words }));
    let got = values(&rules, &group("1"));
    assert_eq!(
        got,
        [(
            "keywords".to_string(),
            Value::List(words.iter().map(|word| text(word)).collect()),
            1
        )]
    );
    // 带控制字符的（换行也算）照规则的规矩整项不收，说是哪一项的。
    for bad in ["e\nf", "\u{1}"] {
        let config = json!({ "im.keywords": ["ok", bad] });
        let parsed = Rules::parse(&[Configured::handed(&config).file()]);
        let named: Vec<_> = parsed
            .problems
            .iter()
            .map(|problem| (problem.code, problem.rule.and_then(key_of)))
            .collect();
        assert_eq!(
            named,
            [(Code::BadFormat, Some("im.keywords".to_string()))],
            "{bad:?}"
        );
    }
}

#[test]
fn a_bad_value_drops_only_that_item_and_names_the_setting() {
    let mut config = everything();
    config["im.sleep"] = json!("abc");
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
            Some("im.sleep".to_string()),
            Some("sleep".to_string())
        )]
    );
    // 别的照用。
    let entries = parsed.rules.resolve(&group("10")).entries;
    assert!(!entries.contains_key("sleep"));
    assert_eq!(entries["show_ids"].value, Value::Bool(true));
}

#[test]
fn rules_name_their_settings_in_order() {
    let keys: Vec<_> = (0..=5).map(key_of).collect();
    assert_eq!(
        keys,
        [
            None,
            Some("im.keywords".to_string()),
            Some("im.sleep".to_string()),
            Some("im.show_ids".to_string()),
            Some("im.strip_period".to_string()),
            None,
        ]
    );
}

#[test]
fn pushed_changes_merge_and_say_whether_anything_moved() {
    let mut configured = Configured::handed(&everything());
    // 别的包的、端口、去掉的四项都不算变。
    let unchanged = json!({"im.qq.listen": 9000, "im.sleep": "23:00-07:00", "other.sleep": "off", "im.groups": [1]});
    assert!(!configured.change(unchanged.as_object().expect("对象")));
    // `null` 回到默认：不看 QQ 号。
    assert!(configured.change(json!({"im.show_ids": null}).as_object().expect("对象")));
    let rules = Rules::parse(&[configured.file()]).rules;
    assert!(!rules.resolve(&group("10")).entries.contains_key("show_ids"));
    // 空的字当没设：不拼进去，也不报写错。
    assert!(configured.change(json!({"im.sleep": ""}).as_object().expect("对象")));
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
    config["im.sleep"] = json!("25:00-26:00");
    config["im.keywords"] = json!(["a\nb", "c\u{1}"]);
    assert_eq!(
        Configured::handed(&config).misread(),
        ["im.keywords", "im.sleep"]
    );
}

#[test]
fn turning_off_the_final_period_reaches_every_venue() {
    // 去句号（施工 O-58 补二，从卡片挪来）：开着是出厂的样子、不写；关了的群和私聊都拼进出站表。
    let rules = rules(json!({"im.strip_period": false}));
    let off = ("outbound.strip_period".to_string(), Value::Bool(false), 4);
    for venue in [group("10"), private("10")] {
        assert_eq!(values(&rules, &venue), std::slice::from_ref(&off));
    }
}
