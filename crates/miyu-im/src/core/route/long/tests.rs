//! 长文：照参数判（转成图、合并转发，阈值不是 0、超过阈值才算，正好等于的不算）；合并转发的节点照拆段的长度拆；页的名字照第几页。拼成的动作在 `im/send/tests.rs`。

use miyu_chat::{File, LongText, Params, Source};

use super::*;

/// 出厂参数：仓库里的 `defaults.toml`。
fn factory() -> Params {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../im/data/defaults.toml");
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
fn pages_are_named_by_their_number() {
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
}
