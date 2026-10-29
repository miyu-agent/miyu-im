//! `fetch_media` 的参数和一轮几次（施工 O-33）：`msg` 是整数或者整数的字，别的当没有这一条；`index` 不写是 1，写错的是 0（取的
//! 时候答「没有这一样」）；一轮最多几次，换了一轮从头数，主线没在跑的也数。取到的那一条要在这个会话里（施工 O-35）：群里是这个
//! 群的，私聊里是这个人在私聊里发的。

use miyu_im::chat::Fetched;
use serde_json::json;

use super::*;

fn want(msg: &str, index: usize) -> Result<Wanted, &'static str> {
    Ok(Wanted {
        msg: msg.to_string(),
        index,
    })
}

#[test]
fn msg_is_an_integer_and_index_counts_from_one() {
    assert_eq!(wanted(&json!({"msg": "8815"})), want("8815", 1));
    assert_eq!(wanted(&json!({"msg": 8815, "index": 2})), want("8815", 2));
    assert_eq!(
        wanted(&json!({"msg": "-2147483000", "index": "3"})),
        want("-2147483000", 3)
    );
    for index in [
        json!(0),
        json!(-1),
        json!(1.5),
        json!("two"),
        json!([1]),
        json!(true),
    ] {
        assert_eq!(
            wanted(&json!({"msg": "1", "index": index.clone()})),
            want("1", 0),
            "{index}"
        );
    }
}

#[test]
fn a_msg_that_is_not_an_integer_is_not_found() {
    for msg in [
        json!(null),
        json!(""),
        json!("-"),
        json!("../8815"),
        json!("8815 "),
        json!("msg=8815"),
        json!("123456789012345678901"),
        json!(1.5),
        json!(u64::MAX),
        json!({"id": 1}),
    ] {
        assert_eq!(
            wanted(&json!({"msg": msg.clone()})),
            Err("not-found"),
            "{msg}"
        );
    }
    assert_eq!(wanted(&json!({})), Err("not-found"));
}

/// 取回来的一条：私聊里的没有群号。
fn fetched(private: bool, group: Option<i64>, user: Option<i64>) -> Fetched {
    Fetched {
        private,
        group: group.map(Id::from),
        user: user.map(Id::from),
        segments: Default::default(),
    }
}

/// 核对归属（施工 O-35）：群里只认这个群的；私聊只认私聊里这个人发的，这个人在群里说的、别人的私聊、她自己发的都不认。号写成字
/// 的由驱动读（`im/replies/tests.rs`）。
#[test]
fn only_a_message_from_this_venue_is_owned() {
    let group = To::Group(Id::from(31));
    assert!(owned(&fetched(false, Some(31), None), group.clone()));
    for other in [
        fetched(false, Some(32), None),
        fetched(true, None, Some(40001)),
        fetched(false, None, None),
    ] {
        assert!(!owned(&other, group.clone()), "{other:?}");
    }
    let private = To::Private(Id::from(40001));
    assert!(owned(&fetched(true, None, Some(40001)), private.clone()));
    for other in [
        fetched(true, None, Some(40002)),
        fetched(false, Some(31), Some(40001)),
        fetched(false, None, Some(40001)),
        fetched(true, None, Some(30003)),
    ] {
        assert!(!owned(&other, private.clone()), "{other:?}");
    }
}
