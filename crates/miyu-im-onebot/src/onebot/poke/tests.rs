//! 拍一拍怎么认（施工 I-248）。

use serde_json::json;

use super::*;
use miyu_im::chat::Id;

/// 一条拍一拍：`user` 拍了 `target`，群号 `group`（私聊的不写），机器人号是 9。
fn poke(user: i64, target: i64, group: Option<i64>) -> Value {
    let mut frame = json!({"post_type": "notice", "notice_type": "notify", "sub_type": "poke", "time": 1_700_000_000,
        "self_id": 9, "user_id": user, "target_id": target});
    if let Some(group) = group {
        frame["group_id"] = json!(group);
    }
    frame
}

#[test]
fn poking_her_in_private_is_a_message_from_the_poker() {
    let Some(Event::Private(posted)) = poked(&poke(5, 9, None)) else {
        panic!("该认成私聊的一条");
    };
    assert_eq!(
        posted.poke,
        Some(Poke { suffix: None }),
        "没带 raw_info 的没有后缀"
    );
    assert_eq!(
        (posted.bot, posted.user, posted.time),
        (Id::from(9), Id::from(5), 1_700_000_000)
    );
    assert!(posted.text.is_empty() && posted.segments == Segments::default());
    assert!(
        miyu_im::chat::is_poke(&posted.message_id.to_string()),
        "编号认得出是戳一戳：{}",
        posted.message_id
    );
}

#[test]
fn poking_her_in_a_group_is_a_group_message() {
    let Some(Event::Group { group, posted }) = poked(&poke(5, 9, Some(31))) else {
        panic!("该认成群里的一条");
    };
    assert_eq!(group, Id::from(31));
    assert!(posted.poke.is_some());
}

#[test]
fn poking_someone_else_in_a_group_is_only_a_happening() {
    let Some(Event::Happened(happened)) = poked(&poke(5, 6, Some(31))) else {
        panic!("该认成群里的一件事");
    };
    assert_eq!(happened.change, GroupChange::Poked);
    assert_eq!(
        (happened.user, happened.operator),
        (Id::from(6), Some(Id::from(5)))
    );
    assert!(poked(&poke(5, 6, None)).is_none(), "私聊里拍别人的不是");
    assert!(poked(&poke(9, 5, Some(31))).is_none(), "她自己拍的不是");
    let mut other = poke(5, 9, None);
    other["sub_type"] = json!("honor");
    assert!(poked(&other).is_none());
}

#[test]
fn the_suffix_is_the_words_after_the_action() {
    let mut frame = poke(5, 9, None);
    frame["raw_info"] = json!([
        {"col": "1", "nm": "", "type": "qq", "uid": "u_a"},
        {"jp": "https://example.invalid", "src": "https://example.invalid/p.png", "type": "img"},
        {"txt": "拍了拍", "type": "nor"},
        {"col": "1", "nm": "", "tp": "0", "type": "qq", "uid": "u_b"},
        {"txt": "的小脑袋", "type": "nor"},
    ]);
    let Some(Event::Private(posted)) = poked(&frame) else {
        panic!("该认成私聊的一条");
    };
    assert_eq!(
        posted.poke.and_then(|poke| poke.suffix).as_deref(),
        Some("的小脑袋")
    );
    frame["raw_info"] = json!([{"txt": "戳了戳", "type": "nor"}, {"txt": " ", "type": "nor"}]);
    let Some(Event::Private(posted)) = poked(&frame) else {
        panic!("该认成私聊的一条");
    };
    assert_eq!(
        posted.poke.and_then(|poke| poke.suffix),
        None,
        "只有动作、空白的没有后缀"
    );
}
