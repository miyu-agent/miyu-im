//! 主动发消息的参数（施工 I-79：号写成字、负数的也认，0 和不合写法的不认）。

use serde_json::json;

use super::send_request;
use crate::chat::{Id, To};

#[test]
fn a_send_names_one_target_by_number_or_text() {
    let to = |args| send_request(&args).map(|(to, _)| to);
    assert_eq!(
        to(json!({"user": 10001, "text": "hi"})),
        Some(To::Private(Id::from(10001)))
    );
    assert_eq!(
        to(json!({"group": " -1001987654321 ", "text": "hi"})),
        Some(To::Group(Id::from(-1_001_987_654_321)))
    );
    assert_eq!(
        to(json!({"user": "wxid_abc", "text": "hi"})),
        Some(To::Private(Id::new("wxid_abc").expect("合写法")))
    );
    for wrong in [
        json!({"user": 0, "text": "hi"}),
        json!({"user": "0", "text": "hi"}),
        json!({"user": "a b", "text": "hi"}),
        json!({"user": 1.5, "text": "hi"}),
        json!({"user": 1, "group": 2, "text": "hi"}),
        json!({"text": "hi"}),
        json!({"user": 1, "text": "  "}),
    ] {
        assert_eq!(to(wrong.clone()), None, "{wrong}");
    }
}
