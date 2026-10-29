//! `chat.action` 的参数怎么读（施工 O-42）：动作名的字符和长度、`params` 不写是空对象、`bot` 只收整数，多写的格不管。

use serde_json::json;

use super::*;

#[test]
fn a_request_reads_name_params_and_bot() {
    assert_eq!(
        Request::read(
            &json!({"action": "some_action", "params": {"group_id": 1}, "bot": 30001, "more": 1})
        ),
        Some(Request {
            action: "some_action".to_string(),
            params: json!({"group_id": 1}).as_object().cloned().expect("对象"),
            bot: Some(Id::from(30001)),
        })
    );
    let bare = Request::read(&json!({"action": ".ocr_image"})).expect("读得出");
    assert_eq!((bare.params.len(), bare.bot), (0, None));
    assert!(
        Request::read(&json!({"action": "_get_model_show", "params": null, "bot": null})).is_some()
    );
    assert!(Request::read(&json!({"action": "a".repeat(NAME_MAX)})).is_some());
}

#[test]
fn a_wrong_request_reads_as_nothing() {
    for wrong in [
        json!({}),
        json!(null),
        json!({"action": ""}),
        json!({"action": "a".repeat(NAME_MAX + 1)}),
        json!({"action": "Get"}),
        json!({"action": "get-group"}),
        json!({"action": "get group"}),
        json!({"action": "群"}),
        json!({"action": 1}),
        json!({"action": "x", "params": []}),
        json!({"action": "x", "params": 1}),
        json!({"action": "x", "bot": "a b"}),
        json!({"action": "x", "bot": "a:b"}),
        json!({"action": "x", "bot": 1.5}),
    ] {
        assert_eq!(Request::read(&wrong), None, "{wrong}");
    }
}
