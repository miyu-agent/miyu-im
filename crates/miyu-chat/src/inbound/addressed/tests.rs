//! 冲她来（`chat.md` 第二条「守着它的」）：私聊、@、引用、触发词在开头、开头有空白、触发词在中间不算、大小写不同不算、
//! 空的触发词不算。

use crate::VenueKind;

use super::addressed;

/// 群里一条没 @ 她、没引用她的消息，正文 `text`，场所的触发词是 `keywords`：冲不冲她来。
fn by_keyword(text: &str, keywords: &[&str]) -> bool {
    let keywords: Vec<String> = keywords.iter().map(|keyword| keyword.to_string()).collect();
    addressed(VenueKind::Group, false, text, &keywords)
}

#[test]
fn private_is_always_addressed() {
    // 私聊里每一条都是对她说的，不看 @、引用、触发词（施工时定的第 17 条）。
    assert!(addressed(VenueKind::Private, false, "hello", &[]));
    assert!(addressed(VenueKind::Private, false, "", &[]));
    let keywords = ["miyu".to_string()];
    assert!(addressed(VenueKind::Private, false, "hello", &keywords));
}

#[test]
fn plain_group_message_is_not_addressed() {
    assert!(!addressed(VenueKind::Group, false, "hello", &[]));
    assert!(!by_keyword("hello", &["miyu"]));
}

#[test]
fn a_mention_is_addressed_and_a_quote_is_not() {
    // 施工 I-20（2026-10-21 项目主人定）：引用她的消息、不 @ 不算叫她。
    assert!(addressed(VenueKind::Group, true, "hello", &[]));
    assert!(addressed(VenueKind::Group, true, "", &[]));
}

#[test]
fn keyword_at_the_start_is_addressed() {
    assert!(by_keyword("miyu 你好", &["miyu"]));
    assert!(by_keyword("miyu", &["miyu"]));
    assert!(by_keyword("miyu酱在吗", &["miyu"]), "后面紧跟着别的字也算");
    assert!(by_keyword("小雨在吗", &["miyu", "小雨"]), "任何一个都行");
    assert!(by_keyword("为什么不查知识库", &["为什么"]));
}

#[test]
fn leading_whitespace_is_skipped() {
    assert!(by_keyword("  miyu 你好", &["miyu"]));
    assert!(by_keyword("\n\tmiyu", &["miyu"]));
    assert!(by_keyword("\u{3000}小雨", &["小雨"]), "全角空格也是空白");
}

#[test]
fn keyword_in_the_middle_is_not_addressed() {
    // 只认开头：中间出现就算，「为什么」这类词会误叫（旧版 `group_trigger_text`，08-29 实测）。
    assert!(!by_keyword("你好 miyu", &["miyu"]));
    assert!(!by_keyword("我想知道为什么", &["为什么"]));
    assert!(!by_keyword("，miyu", &["miyu"]), "标点不是空白");
}

#[test]
fn case_matters() {
    assert!(!by_keyword("Miyu 你好", &["miyu"]));
    assert!(!by_keyword("miyu 你好", &["Miyu"]));
    assert!(!by_keyword("MIYU", &["miyu"]));
}

#[test]
fn empty_keyword_does_not_count() {
    assert!(!by_keyword("hello", &[""]));
    assert!(!by_keyword("", &[""]));
    assert!(by_keyword("miyu", &["", "miyu"]), "空的不算，别的照看");
    // 触发词不去空白：写成 ` miyu` 的，正文去掉开头的空白以后对不上。
    assert!(!by_keyword(" miyu", &[" miyu"]));
}

#[test]
fn a_call_with_nothing_else_is_hollow() {
    // 施工 I-29：去掉触发词、@ 开头的词以后，没有字母、数字、汉字这类字的是只叫了她。
    let keywords = vec!["miyu".to_string(), "米尤".to_string()];
    for text in [
        "miyu",
        "miyu？",
        "@米尤",
        "@米尤 😭",
        "米尤 ！！",
        "",
        "miyu miyu",
    ] {
        assert!(super::hollow(text, &keywords), "{text}");
    }
    for text in ["miyu 在吗", "@米尤 帮我看看", "miyu似了", "@阿甲 @米尤 3"] {
        assert!(!super::hollow(text, &keywords), "{text}");
    }
}
