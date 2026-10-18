//! `mention` 的参数（施工 O-55）：正整数、写成字的也认，去重照先后，1 到 10 个；别的不收。

use serde_json::json;

use super::users;

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
    let eleven: Vec<i64> = (1..=11).collect();
    assert_eq!(users(&json!({"users": eleven})), None, "最多十个");
}
