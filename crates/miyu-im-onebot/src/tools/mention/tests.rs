//! `mention` 的参数（施工 O-55）：正整数、写成字的也认，去重照先后，1 到 32 个（施工 I-17）；别的不收。认成员照缓存、成员表，一个
//! 不是成员不连累别的。

use std::collections::HashMap;

use serde_json::json;

use super::{named, users};

#[test]
fn the_users_are_read_and_deduplicated() {
    assert_eq!(
        users(&json!({"users": [40001, "40002", 40001]})),
        Some(vec![40001, 40002])
    );
    assert_eq!(users(&json!({"users": []})), None, "一个都没有");
    assert_eq!(
        users(&json!({"users": [40001, 0]})),
        None,
        "有写错的整个不收"
    );
    assert_eq!(users(&json!({"users": ["小明"]})), None);
    assert_eq!(users(&json!({"users": 40001})), None, "不是列表");
    let most: Vec<i64> = (1..=32).collect();
    assert_eq!(
        users(&json!({"users": most})).map(|users| users.len()),
        Some(32)
    );
    let more: Vec<i64> = (1..=33).collect();
    assert_eq!(users(&json!({"users": more})), None, "最多 32 个");
}

#[test]
fn members_come_from_the_cache_or_the_list_and_strangers_stand_alone() {
    let cached = HashMap::from([(40001, "小明".to_string())]);
    let listed = HashMap::from([(40002, Some("阿红".to_string())), (40003, None)]);
    // 头一个不是成员：后面的照样认（施工 I-17：以前问不到一次以后的都当不是成员）。
    assert_eq!(
        named(&[40009, 40001, 40002, 40003], &cached, &listed),
        (
            vec!["小明".to_string(), "阿红".to_string(), "40003".to_string()],
            vec!["40009".to_string()]
        )
    );
}
