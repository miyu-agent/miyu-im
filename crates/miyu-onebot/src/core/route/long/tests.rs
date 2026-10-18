//! 长文：照参数判（转成图、合并转发，阈值不是 0、超过阈值才算，正好等于的不算）；合并转发的节点照拆段的长度拆、拼成转发的动作；画成的一页带引用和 @ 拼在图前面；页的名字照第几页。

use miyu_chat::{File, LongText, Params, Source};

use super::*;
use crate::chat::{Lead, To};
use crate::onebot::picture_to;

/// 出厂参数：仓库里的 `defaults.toml`。
fn factory() -> Params {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../onebot/data/defaults.toml"
    );
    let file = File {
        source: Source::Factory,
        name: "defaults.toml".to_string(),
        text: std::fs::read_to_string(path).expect("出厂的读得到"),
    };
    Params::read(&file).expect("读得出")
}

/// 出厂参数，长文两格换成写的。
fn params(long_text: LongText, long_chars: usize) -> Params {
    let mut params = factory();
    params.long_text = long_text;
    params.long_chars = long_chars;
    params
}

#[test]
fn only_image_mode_over_the_threshold_is_drawn() {
    let text = "字".repeat(300);
    assert!(
        !wants(&params(LongText::Image, 300), &text).is_some(),
        "正好 300 不算"
    );
    assert!(wants(&params(LongText::Image, 299), &text) == Some(LongText::Image));
    assert!(
        !wants(&params(LongText::Split, 299), &text).is_some(),
        "拆成几条的不画"
    );
    assert!(
        !wants(&params(LongText::Image, 0), &text).is_some(),
        "0 是不算长文"
    );
    let factory = factory();
    assert_eq!(
        (factory.long_text, factory.long_chars),
        (LongText::Image, 300),
        "出厂转成图、300"
    );
}

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
        serde_json::json!([
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
        serde_json::json!([{"type": "image", "data": {"file": "base64://AA=="}}])
    );
    assert_eq!(page_name(1), "long-text-2.png");
}

#[test]
fn forward_mode_is_long_text_too_and_its_nodes_follow_split_chars() {
    let text = format!("{}\n\n{}", "一".repeat(200), "二".repeat(200));
    assert_eq!(
        wants(&params(LongText::Forward, 300), &text),
        Some(LongText::Forward)
    );
    let mut small = params(LongText::Forward, 300);
    small.split_chars = 250;
    assert_eq!(
        nodes(&small, &text),
        ["一".repeat(200), "二".repeat(200)],
        "照拆成几条的长度拆节点"
    );
    let (action, params) =
        crate::onebot::forward_to(To::Private(9), 42, "米尤", &["甲".to_string()]);
    assert_eq!(action, "send_private_forward_msg");
    assert_eq!(
        params,
        serde_json::json!({"user_id": 9, "messages": [{"type": "node", "data": {"user_id": "42", "nickname": "米尤", "content": [{"type": "text", "data": {"text": "甲"}}]}}]})
    );
}
