//! 场所的格照核心的写法洗（`onebot.md` 第一条「群消息」第 6 条，「施工时定的」第 63 条）：名字去掉控制字符、截到上限，空白
//! 的不写；引用、带的东西的编号不合的那一格、那一样不记，消息照记；假的开关不写。交了原图的那几样记 `shown`（施工 O-35）。

use serde_json::json;

use super::{Flags, MEDIA_NAME, NAME, fields, shown};
use crate::chat::{Fetch, Media, MediaKind, Platform, Posted, Segments};

/// 测试用的平台。
const QQ: Platform = Platform::new("qq");

/// 一条消息：名字是 `name`，认出来的段是 `segments`。
fn posted(name: Option<&str>, segments: Segments) -> Posted {
    Posted {
        bot: 30003,
        user: 20002,
        message_id: 9,
        time: 0,
        name: name.map(str::to_string),
        rank: None,
        text: String::new(),
        segments,
    }
}

/// 一样带的东西：没有大小。
fn media(id: &str, name: Option<&str>) -> Media {
    Media {
        kind: MediaKind::File,
        id: id.to_string(),
        name: name.map(str::to_string),
        size: None,
        fetch: Some(Fetch::File),
    }
}

/// 施工 O-33：带的东西有大小的写 `size`，0 也写；没有的不写。
#[test]
fn a_size_is_written_when_known() {
    let segments = Segments {
        media: vec![
            Media {
                size: Some(1_234_567),
                ..media("f-1", Some("a.pdf"))
            },
            Media {
                size: Some(0),
                ..media("f-2", None)
            },
            media("f-3", None),
        ],
        ..Segments::default()
    };
    assert_eq!(
        fields(&posted(None, segments), &[], Flags::default(), None, None)["media"],
        json!([
            {"kind": "file", "id": "f-1", "name": "a.pdf", "size": 1_234_567},
            {"kind": "file", "id": "f-2", "size": 0},
            {"kind": "file", "id": "f-3"},
        ])
    );
}

#[test]
fn only_what_is_there_is_written() {
    assert_eq!(
        fields(
            &posted(None, Segments::default()),
            &[],
            Flags::default(),
            None,
            None
        ),
        json!({"msg": "9"})
    );
    let all = Flags {
        mentions_me: true,
        ambient: true,
        asleep: true,
        show_ids: true,
    };
    let segments = Segments {
        reply_to: Some("8".to_string()),
        mentions_all: true,
        media: vec![media("f-1", Some("a.pdf"))],
        ..Segments::default()
    };
    let mentions = [QQ.person(20003).expect("拼得出")];
    assert_eq!(
        fields(
            &posted(Some("小林"), segments),
            &mentions,
            all,
            Some("美羽"),
            Some("摸鱼群")
        ),
        json!({
            "msg": "9", "name": "小林", "my_name": "美羽", "place": "摸鱼群", "reply_to": "8", "mentions": ["qq:20003"], "mentions_me": true,
            "mentions_all": true, "media": [{"kind": "file", "id": "f-1", "name": "a.pdf"}], "ambient": true,
            "asleep": true, "show_ids": true,
        })
    );
}

#[test]
fn names_lose_control_characters_and_are_cut() {
    let long = "名".repeat(NAME + 6);
    let written = |name: &str| {
        fields(
            &posted(Some(name), Segments::default()),
            &[],
            Flags::default(),
            Some(name),
            Some(name),
        )
    };
    // 她在这个场所叫什么（施工 I-38）、群名（施工 I-54）照同一个规矩洗。
    for key in ["name", "my_name", "place"] {
        assert_eq!(written(&long)[key], "名".repeat(NAME));
        assert_eq!(written("小\u{0}林\n")[key], "小林");
        assert_eq!(written(" 小林 ")[key], " 小林 ", "照原样，只去控制字符");
        for blank in ["", "  ", "\u{7}\u{1b}", " \n "] {
            assert!(written(blank).get(key).is_none(), "{key} {blank:?}");
        }
    }
    let segments = Segments {
        media: vec![
            media("f-1", Some(&"文".repeat(MEDIA_NAME + 1))),
            media("f-2", Some(" \t")),
        ],
        ..Segments::default()
    };
    let written = fields(&posted(None, segments), &[], Flags::default(), None, None);
    assert_eq!(
        written["media"],
        json!([{"kind": "file", "id": "f-1", "name": "文".repeat(MEDIA_NAME)}, {"kind": "file", "id": "f-2"}])
    );
}

#[test]
fn ids_that_do_not_fit_are_left_out_and_the_message_is_kept() {
    let longest = "1".repeat(128);
    for (reply_to, kept) in [
        (longest.clone(), true),
        ("1".repeat(129), false),
        ("8\n".to_string(), false),
        (String::new(), false),
    ] {
        let segments = Segments {
            reply_to: Some(reply_to.clone()),
            ..Segments::default()
        };
        let written = fields(&posted(None, segments), &[], Flags::default(), None, None);
        assert_eq!(written.get("reply_to").is_some(), kept, "{reply_to:?}");
        assert_eq!(written["msg"], "9");
    }
    let segments = Segments {
        media: vec![
            media(&"x".repeat(129), None),
            media(&longest, None),
            media("a\u{0}", None),
        ],
        ..Segments::default()
    };
    let written = fields(&posted(None, segments), &[], Flags::default(), None, None);
    assert_eq!(written["media"], json!([{"kind": "file", "id": longest}]));
    let segments = Segments {
        media: vec![media(&"x".repeat(129), None)],
        ..Segments::default()
    };
    assert!(
        fields(&posted(None, segments), &[], Flags::default(), None, None)
            .get("media")
            .is_none(),
        "一样都不剩的不写"
    );
}

/// 交了原图的那几样记 `shown`（施工 O-35）：照第几个（从 1 数）找到 `media` 里那一样；没有那一样的、没有 `media` 的不动。
#[test]
fn shown_marks_the_items_by_their_number() {
    let mut venue = json!({"msg": "9", "media": [{"kind": "image", "id": "a"}, {"kind": "file", "id": "b"}, {"kind": "image", "id": "c"}]});
    shown(&mut venue, &[1, 3, 4, 0]);
    assert_eq!(
        venue["media"],
        json!([{"kind": "image", "id": "a", "shown": true}, {"kind": "file", "id": "b"}, {"kind": "image", "id": "c", "shown": true}])
    );
    let mut bare = json!({"msg": "9"});
    shown(&mut bare, &[1]);
    assert_eq!(bare, json!({"msg": "9"}));
}
