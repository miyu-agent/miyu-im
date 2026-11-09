use serde_json::json;

use super::{Rows, index_of};
use crate::driver::TokenTable;

const TABLE: TokenTable = TokenTable {
    tokens: "tokens",
    columns: &[("proxies", "proxy", "env")],
};

#[test]
fn the_columns_are_written_as_long_as_the_tokens() {
    let rows = Rows {
        tokens: vec![json!({"secret": "a"}), json!({"secret": "b"})],
        columns: vec![vec![json!("direct")]],
    };
    assert_eq!(
        rows.changes("tg", &TABLE),
        [
            json!({"key": "im.tg.tokens", "value": [{"secret": "a"}, {"secret": "b"}]}),
            json!({"key": "im.tg.proxies", "value": ["direct", "env"]}),
        ]
    );
}

#[test]
fn a_bot_is_found_by_what_the_driver_reported() {
    let status =
        json!({"connections": [{"index": 0, "bot": "7"}, {"index": 2, "bot": "9"}, {"index": 1}]});
    let status = status.as_object().expect("对象");
    assert_eq!(index_of(status, "9"), Some(2));
    assert_eq!(index_of(status, "8"), None);
}
