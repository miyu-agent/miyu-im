//! 合并转发里的一条怎么读（施工 O-34）：NapCat 回的完整消息、发合并转发那种 `node` 写法都认；名字先群名片、再昵称；号先
//! `sender.user_id`；段照 `message`，没有的照 `content`，字符串照 CQ 码只读字。

use miyu_im::chat::Id;
use serde_json::json;

use super::*;
use crate::onebot::Piece;

#[test]
fn a_napcat_message_node_is_read() {
    let read = node(&json!({
        "message_id": 1, "user_id": "20017", "time": 1,
        "sender": {"user_id": 20017, "nickname": "阿杰", "card": "  "},
        "message": [{"type": "text", "data": {"text": "嗯"}}, {"type": "image", "data": {"file": "a.jpg"}}],
    }));
    assert_eq!(read.user, Some(Id::from(20017)));
    assert_eq!(read.name.as_deref(), Some("阿杰"), "群名片空白的照昵称");
    assert_eq!(read.segments.pieces, [Piece::Text("嗯".to_string())]);
    assert_eq!(read.segments.media.len(), 1);
}

#[test]
fn a_send_style_node_and_a_cq_string_are_read() {
    let read = node(&json!({"type": "node", "data": {
        "user_id": "30003", "nickname": "老王", "content": "[CQ:face,id=1]你好",
    }}));
    assert_eq!(read.user, Some(Id::from(30003)));
    assert_eq!(read.name.as_deref(), Some("老王"));
    assert_eq!(read.segments.pieces, [Piece::Text("你好".to_string())]);
    let nameless = node(&json!({"message": []}));
    assert_eq!((nameless.user, nameless.name), (None, None));
}

#[test]
fn the_action_and_the_reply() {
    assert_eq!(
        get_forward_msg("7495"),
        ("get_forward_msg", json!({"id": "7495"}))
    );
    assert_eq!(
        forward_messages(&json!({"messages": [{"a": 1}]})),
        Some(vec![json!({"a": 1})])
    );
    assert_eq!(forward_messages(&json!({"messages": "x"})), None);
    assert_eq!(forward_messages(&json!(null)), None);
}

/// QQ「聊天记录」卡片，照 NapCat 给转发里再套的转发那样（`data.data` 是卡片 JSON 的字）。
fn chat_record(resid: &str) -> serde_json::Value {
    let card = json!({"app": "com.tencent.multimsg", "desc": "[聊天记录]",
        "meta": {"detail": {"news": [{"text": "Shorin: 嗯"}], "resid": resid, "summary": "查看2条转发消息"}}});
    json!({"type": "json", "data": {"data": card.to_string()}})
}

#[test]
fn a_chat_record_card_inside_a_forward_is_a_forward_to_fetch() {
    let other = json!({"type": "json", "data": {"data": r#"{"app":"com.tencent.miniapp_01"}"#}});
    let read = node(&json!({"user_id": 20017, "sender": {"nickname": "阿杰"},
        "message": [chat_record("res/abc=="), other, {"type": "json", "data": {"data": "{"}}]}));
    assert_eq!(
        read.segments.pieces,
        ["[forward]", "[card]", "[card]"].map(|text| Piece::Text(text.to_string())),
        "聊天记录卡片写成转发的占位（施工 I-237），别的卡片、读不出的照旧"
    );
    assert_eq!(read.segments.forwards.len(), 1);
    assert_eq!(read.segments.forwards[0].id.as_deref(), Some("res/abc=="));
    assert!(
        read.segments.forwards[0].content.is_none(),
        "照资源编号去取"
    );
    let object = json!({"type": "json", "data": {"data": {"app": "com.tencent.multimsg",
        "meta": {"detail": {"resid": "r2"}}}}});
    let read = node(&json!({"message": [object]}));
    assert_eq!(
        read.segments.forwards[0].id.as_deref(),
        Some("r2"),
        "对象也认"
    );
}
