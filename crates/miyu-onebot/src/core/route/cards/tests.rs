//! 后台页的场所卡片（施工 O-58 上）：NapCat 回的群、好友的表怎么读。整条路（存、读、当场换上）在 `tests/cards.rs`。

use serde_json::json;

use super::entries;

#[test]
fn rows_keep_number_name_and_count_once_each() {
    let data = json!([
        {"group_id": 777, "group_name": "测试群", "member_count": 12},
        {"group_id": "x", "group_name": "号不是数"},
        {"group_id": 777, "group_name": "另一个号也在这个群", "member_count": 12},
        {"group_id": 666, "member_count": 3},
    ]);
    assert_eq!(
        entries(&data, "group_id", "group_name", Some("member_count")),
        [
            json!({"id": 777, "name": "测试群", "members": 12}),
            json!({"id": 666, "name": null, "members": 3}),
        ]
    );
    let friends = json!([{"user_id": 1, "nickname": "a"}]);
    assert_eq!(
        entries(&friends, "user_id", "nickname", None),
        [json!({"id": 1, "name": "a"})]
    );
    assert_eq!(
        entries(&json!(null), "user_id", "nickname", None),
        Vec::<serde_json::Value>::new()
    );
}
