//! 后台页的场所卡片（施工 O-58 上）：几个号回的群、好友的表怎么并。表怎么从 NapCat 的回应读在 `im/replies/tests.rs`；
//! 整条路（存、读、当场换上）在 `tests/cards.rs`。

use serde_json::{Value, json};

use super::entries;
use crate::chat::Id;

#[test]
fn rows_keep_the_first_of_each_number() {
    let rows = [
        (
            Id::from(777),
            json!({"id": 777, "name": "测试群", "members": 12}),
        ),
        (
            Id::from(777),
            json!({"id": 777, "name": "另一个号也在这个群", "members": 12}),
        ),
        (
            Id::from(666),
            json!({"id": 666, "name": null, "members": 3}),
        ),
    ];
    assert_eq!(
        entries(rows.into_iter()),
        [
            json!({"id": 777, "name": "测试群", "members": 12}),
            json!({"id": 666, "name": null, "members": 3}),
        ]
    );
    assert_eq!(entries(std::iter::empty()), Vec::<Value>::new());
}
