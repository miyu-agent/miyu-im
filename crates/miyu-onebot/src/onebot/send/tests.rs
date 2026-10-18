//! 发消息拼成的动作（施工 O-59，从 `core/route/long/tests.rs` 挪来）：长文画成的一页，引用和 @ 拼在图前面；合并转发的节点照
//! 机器人号、名字拼。

use serde_json::json;

use super::{forward_to, picture_to};
use crate::chat::{Lead, To};

#[test]
fn a_page_carries_the_quote_and_mentions_before_the_image() {
    let lead = Lead {
        reply: Some("77".into()),
        at: Some("1001".into()),
        more: vec!["1001".into(), "1002".into()],
    };
    let (action, params) = picture_to(To::Group(5), "base64://AA==", &lead);
    assert_eq!(action, "send_group_msg");
    assert_eq!(
        params["message"],
        json!([
            {"type": "reply", "data": {"id": "77"}},
            {"type": "at", "data": {"qq": "1001"}},
            {"type": "text", "data": {"text": " "}},
            {"type": "at", "data": {"qq": "1002"}},
            {"type": "text", "data": {"text": " "}},
            {"type": "image", "data": {"file": "base64://AA=="}},
        ])
    );
    let (action, params) = picture_to(To::Private(9), "base64://AA==", &Lead::default());
    assert_eq!(action, "send_private_msg");
    assert_eq!(params["user_id"], 9);
    assert_eq!(
        params["message"],
        json!([{"type": "image", "data": {"file": "base64://AA=="}}])
    );
}

#[test]
fn a_forward_is_one_node_per_piece() {
    let (action, params) = forward_to(To::Private(9), 42, "米尤", &["甲".to_string()]);
    assert_eq!(action, "send_private_forward_msg");
    assert_eq!(
        params,
        json!({"user_id": 9, "messages": [{"type": "node", "data": {"user_id": "42", "nickname": "米尤", "content": [{"type": "text", "data": {"text": "甲"}}]}}]})
    );
}
