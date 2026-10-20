//! 通讯录的参数、列法（施工 I-12）。

use serde_json::json;

use super::*;

#[test]
fn the_kind_is_friends_unless_groups_is_asked() {
    assert_eq!(kind(&json!({})), Some(Kind::Friends));
    assert_eq!(kind(&json!({"kind": "groups"})), Some(Kind::Groups));
    assert_eq!(kind(&json!({"kind": "people"})), None);
}

#[test]
fn people_are_found_by_name_or_number_and_capped() {
    let people = [
        (10001, Some("小明".to_string())),
        (10002, Some("阿\n杰".to_string())),
        (20003, None),
    ];
    assert_eq!(
        listed(&people, None, 50),
        (3, 3, "10001 小明; 10002 阿杰; 20003".to_string())
    );
    assert_eq!(
        listed(&people, Some(" 小明 "), 50),
        (1, 1, "10001 小明".to_string())
    );
    assert_eq!(
        listed(&people, Some("2000"), 50),
        (1, 1, "20003".to_string())
    );
    assert_eq!(listed(&people, None, 2).0, 2);
    assert_eq!(listed(&people, None, 2).1, 3);
}
