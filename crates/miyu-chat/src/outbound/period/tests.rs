//! 去末尾句号（`chat.md` 第五条「守着它的」，施工 O-47）：去一个、只去中文句号、末尾空白不算、只有句号的不去、关了不去、
//! 中间的不动。

use super::super::Out;
use super::super::test_support::{ctx, judge, text};

/// 过整条链（`ctx()` 的情形：这一回合什么都没发过），`strip` 是开关，交回发出去的正文。
fn sent(body: &str, strip: bool) -> String {
    let mut ctx = ctx();
    ctx.outbound.strip_period = strip;
    match judge(text(body), &ctx) {
        Out::Send { outgoing, .. } => outgoing.text,
        Out::Drop(why) => panic!("dropped: {why:?}"),
    }
}

#[test]
fn one_trailing_period_goes() {
    assert_eq!(sent("好的。", true), "好的");
    assert_eq!(sent("好的。。", true), "好的。");
    assert_eq!(sent("好的……。", true), "好的……");
}

#[test]
fn trailing_blank_does_not_hide_it() {
    assert_eq!(sent("好的。 \n", true), "好的");
}

#[test]
fn only_the_chinese_period() {
    assert_eq!(sent("好的.", true), "好的.");
    assert_eq!(sent("好的！", true), "好的！");
    assert_eq!(sent("好的｡", true), "好的｡");
}

#[test]
fn middle_periods_stay() {
    assert_eq!(sent("第一句。第二句。", true), "第一句。第二句");
    assert_eq!(sent("第一句。\n第二句", true), "第一句。\n第二句");
}

#[test]
fn a_lone_period_is_kept() {
    assert_eq!(sent("。", true), "。");
    assert_eq!(sent("  。 ", true), "  。 ");
}

#[test]
fn off_keeps_it() {
    assert_eq!(sent("好的。", false), "好的。");
}
