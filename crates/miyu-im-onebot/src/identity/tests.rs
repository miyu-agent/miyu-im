//! 识人术的比法（施工 O-53，I-10 上挪进驱动）：小写、空白不算、含着就算，一个字的不比，几个都借的取头一个，没借的没有。

use super::borrowed;

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|one| (*one).to_string()).collect()
}

#[test]
fn a_name_containing_a_protected_one_borrows_it() {
    let protected = names(&["老板", "Miyu Bot", "米尤"]);
    assert_eq!(
        borrowed("老板", &protected).map(String::as_str),
        Some("老板")
    );
    assert_eq!(
        borrowed("老板本人", &protected).map(String::as_str),
        Some("老板")
    );
    assert_eq!(
        borrowed("我是 老 板", &protected).map(String::as_str),
        Some("老板"),
        "空白不算"
    );
    assert_eq!(
        borrowed("miyubot", &protected).map(String::as_str),
        Some("Miyu Bot"),
        "大小写、空白都不算"
    );
    assert_eq!(
        borrowed("米尤老板", &protected).map(String::as_str),
        Some("老板"),
        "照名单的先后取头一个"
    );
    assert_eq!(borrowed("小林", &protected), None);
}

#[test]
fn a_one_letter_name_is_not_compared() {
    let protected = names(&["A", "  好  "]);
    assert_eq!(borrowed("Alice", &protected), None);
    assert_eq!(borrowed("你好", &protected), None);
}

/// 群里的一条：`name` 说的、号是 `number`，终端管理员用过「老板」、她的群名片是「米尤」。
fn seen<'a>(name: &'a str, number: &'a str, admin: &'a [String]) -> miyu_im::plugins::Seen<'a> {
    miyu_im::plugins::Seen {
        name,
        number,
        admin_names: admin,
        own_card: Some("米尤"),
    }
}

#[test]
fn each_protection_follows_its_switch_and_the_people_map() {
    use miyu_im::plugins::Builtin;
    use serde_json::{Map, Value, json};

    use super::{ADMIN, CARD, Identity, PEOPLE};

    let admin = names(&["老板"]);
    let config =
        |pairs: Value| -> Map<String, Value> { pairs.as_object().cloned().unwrap_or_default() };
    let all_on =
        config(json!({PEOPLE: ["阿杰=10002", "阿杰=10003", "坏的", "小王=abc", "=10009"]}));
    let claimed = |config: &Map<String, Value>, name, number| {
        Identity.impostor(config, &seen(name, number, &admin))
    };
    assert_eq!(
        claimed(&all_on, "老板本人", "20001").as_deref(),
        Some("老板")
    );
    assert_eq!(
        claimed(&all_on, "米尤bot", "20001").as_deref(),
        Some("米尤")
    );
    assert_eq!(
        claimed(&all_on, "我是阿杰", "20001").as_deref(),
        Some("阿杰")
    );
    assert_eq!(claimed(&all_on, "阿杰", "10002"), None, "本人");
    assert_eq!(
        claimed(&all_on, "阿杰小号", "10003"),
        None,
        "同一个名字写了几个号，哪一个都算本人"
    );
    assert_eq!(claimed(&all_on, "小王", "1"), None, "写错的那一项不算");
    let off = config(json!({ADMIN: false, CARD: false}));
    assert_eq!(claimed(&off, "老板本人", "20001"), None);
    assert_eq!(claimed(&off, "米尤bot", "20001"), None);
}

#[test]
fn the_people_map_reads_name_equals_number() {
    use serde_json::json;

    use super::people;

    let read = people(Some(&json!([
        " 主人 = 10001 ",
        "x=1",
        "坏的",
        "a=b",
        7,
        "=1"
    ])));
    assert_eq!(
        read,
        [
            ("主人".to_string(), "10001".to_string()),
            ("x".to_string(), "1".to_string())
        ]
    );
    assert!(people(None).is_empty());
    assert!(people(Some(&json!("主人=1"))).is_empty());
}
