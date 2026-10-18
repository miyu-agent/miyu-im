//! `chat_send`、`chat.send` 的参数（施工 O-56）：发到哪写正好一个、正整数（字也认），发什么不空、不超过四千个字符。

use serde_json::json;

use super::request;
use crate::onebot::To;

#[test]
fn where_and_what_are_read() {
    assert_eq!(
        request(&json!({"group": 888, "text": " 大家好 "})),
        Some((To::Group(888), "大家好".to_string()))
    );
    assert_eq!(
        request(&json!({"user": "40001", "text": "在吗"})),
        Some((To::Private(40001), "在吗".to_string()))
    );
    for args in [
        json!({"user": 1, "group": 2, "text": "x"}),
        json!({"text": "x"}),
        json!({"user": 0, "text": "x"}),
        json!({"user": 1, "text": "  "}),
        json!({"user": 1}),
        json!({"user": 1, "text": "字".repeat(4001)}),
    ] {
        assert_eq!(request(&args), None, "{args}");
    }
    assert!(
        request(&json!({"user": 1, "text": "字".repeat(4000)})).is_some(),
        "正好四千个照收"
    );
}
