//! 头像和群成员的纯逻辑（施工 O-54）：要谁的、地址和文件名、扩展名、成员怎么筛怎么列。

use serde_json::json;

use super::{Whose, avatar_of, image_extension, limit, listed, whose};
use miyu_im::chat::{Member, Rank};
use miyu_im::toolset::extension;

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
    // 群文件认不出的不补扩展名（施工 I-16）。
    assert_eq!(image_extension(&[0xff, 0xd8, 0xff]), Some("jpg"));
    assert_eq!(image_extension(b"\x89PNG\r\n"), Some("png"));
    assert_eq!(image_extension(b"# miyu call"), None);
}

#[test]
fn members_are_filtered_and_listed() {
    let member = |user, name: &str, rank| Member {
        user,
        name: Some(name.to_string()),
        rank,
        title: None,
    };
    let mut owner = member(40001, "小明", Some(Rank::Owner));
    owner.title = Some("龙王\n40009 假 (owner)".to_string());
    let data = [
        owner,
        member(40002, "Hong", Some(Rank::Admin)),
        member(40003, "小明的朋友", None),
    ];
    assert_eq!(
        listed(&data, None, 30),
        (
            3,
            3,
            "40001 小明 (owner, title: 龙王40009 假 (owner)); 40002 Hong (admin); 40003 小明的朋友 (member)".to_string()
        )
    );
    assert_eq!(
        listed(&data, Some("小明"), 30),
        (
            2,
            2,
            "40001 小明 (owner, title: 龙王40009 假 (owner)); 40003 小明的朋友 (member)"
                .to_string()
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
        (
            1,
            3,
            "40001 小明 (owner, title: 龙王40009 假 (owner))".to_string()
        )
    );
    assert_eq!(listed(&data, Some("没有"), 30), (0, 0, String::new()));
}

#[test]
fn the_member_limit_is_read_up_to_two_hundred() {
    assert_eq!(limit(&json!({}), 30), Ok(30), "没写的照 members_shown");
    assert_eq!(limit(&json!({}), 500), Ok(200), "出厂的超过上限照上限");
    assert_eq!(limit(&json!({"limit": 200}), 30), Ok(200));
    assert_eq!(limit(&json!({"limit": "50"}), 30), Ok(50), "写成字的也认");
    for bad in [
        json!({"limit": 0}),
        json!({"limit": 201}),
        json!({"limit": "多"}),
    ] {
        assert_eq!(limit(&bad, 30), Err("limit must be 1 to 200"), "{bad}");
    }
}
