//! 冒充提醒的比法（施工 O-53）：小写、空白不算、含着就算，一个字的不比，几个都借的取头一个，没借的没有。

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
