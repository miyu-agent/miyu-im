//! 头像和群成员的纯逻辑（施工 O-54）：要谁的、地址和文件名、扩展名、成员怎么筛怎么列。

use serde_json::json;

use super::{Whose, avatar_of, extension, listed, whose};
use crate::chat::{Member, Rank};

#[test]
fn whose_avatar_is_read_from_the_arguments_or_the_caller() {
    assert_eq!(
        whose(&json!({"user": 40001}), Some(1)),
        Some(Whose::User(40001))
    );
    assert_eq!(
        whose(&json!({"user": " 40001 "}), None),
        Some(Whose::User(40001))
    );
    assert_eq!(
        whose(&json!({"group": 888}), Some(1)),
        Some(Whose::Group(888))
    );
    assert_eq!(
        whose(&json!({}), Some(10001)),
        Some(Whose::User(10001)),
        "都没写的取叫她的人"
    );
    assert_eq!(whose(&json!({}), None), None);
    assert_eq!(
        whose(&json!({"user": 1, "group": 2}), Some(3)),
        None,
        "两个都写了"
    );
    assert_eq!(whose(&json!({"user": 0}), Some(3)), None);
    assert_eq!(whose(&json!({"user": "小明"}), Some(3)), None);
}

#[test]
fn the_address_and_name_follow_the_templates() {
    let templates = (
        "https://a/{id}/x".to_string(),
        "https://g/{id}/{id}/".to_string(),
    );
    assert_eq!(
        avatar_of(Whose::User(5), &templates),
        ("https://a/5/x".to_string(), "avatar-qq-5".to_string())
    );
    assert_eq!(
        avatar_of(Whose::Group(7), &templates),
        ("https://g/7/7/".to_string(), "avatar-group-7".to_string())
    );
}

#[test]
fn the_extension_is_read_from_the_bytes() {
    assert_eq!(extension(b"\x89PNG\r\n"), "png");
    assert_eq!(extension(b"GIF89a"), "gif");
    assert_eq!(extension(b"RIFF\0\0\0\0WEBPVP8 "), "webp");
    assert_eq!(extension(&[0xff, 0xd8, 0xff]), "jpg");
}

#[test]
fn members_are_filtered_and_listed() {
    let member = |user, name: &str, rank| Member {
        user,
        name: Some(name.to_string()),
        rank,
    };
    let data = [
        member(40001, "小明", Some(Rank::Owner)),
        member(40002, "Hong", Some(Rank::Admin)),
        member(40003, "小明的朋友", None),
    ];
    assert_eq!(
        listed(&data, None, 30),
        (
            3,
            3,
            "40001 小明 (owner); 40002 Hong (admin); 40003 小明的朋友 (member)".to_string()
        )
    );
    assert_eq!(
        listed(&data, Some("小明"), 30),
        (
            2,
            2,
            "40001 小明 (owner); 40003 小明的朋友 (member)".to_string()
        )
    );
    assert_eq!(
        listed(&data, Some("hong"), 30).2,
        "40002 Hong (admin)",
        "不分大小写"
    );
    assert_eq!(
        listed(&data, Some("40003"), 30).2,
        "40003 小明的朋友 (member)",
        "照号也找"
    );
    assert_eq!(
        listed(&data, None, 1),
        (1, 3, "40001 小明 (owner)".to_string())
    );
    assert_eq!(listed(&data, Some("没有"), 30), (0, 0, String::new()));
}
