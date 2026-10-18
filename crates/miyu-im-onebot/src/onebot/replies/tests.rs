//! 读 NapCat 的回应（施工 I-2 上）：号是整数或者写成整数的字，读不出的行不要；名字照原样。

use serde_json::json;

use super::{fetched, friends, group_name, groups, member, members};
use miyu_im::chat::{Friend, Group, Member, Rank};

#[test]
fn group_and_friend_lists_keep_rows_with_a_number() {
    let data = json!([
        {"group_id": 777, "group_name": "测试群", "member_count": 12},
        {"group_id": "x", "group_name": "号不是数"},
        {"group_id": "666", "member_count": 3},
    ]);
    assert_eq!(
        groups(&data),
        [
            Group {
                id: 777,
                name: Some("测试群".into()),
                members: Some(12),
            },
            Group {
                id: 666,
                name: None,
                members: Some(3),
            },
        ]
    );
    assert_eq!(
        friends(&json!([{"user_id": 1, "nickname": "a"}, {"nickname": "没号"}])),
        [Friend {
            id: 1,
            name: Some("a".into()),
        }]
    );
    assert!(groups(&json!(null)).is_empty());
}

#[test]
fn members_read_card_before_nickname_and_the_role() {
    let data = json!([
        {"user_id": 1, "card": "小明", "nickname": "ming", "role": "owner"},
        {"user_id": 2, "card": "  ", "nickname": "hong", "role": "guest"},
        {"card": "没号"},
    ]);
    assert_eq!(
        members(&data),
        [
            Member {
                user: 1,
                name: Some("小明".into()),
                rank: Some(Rank::Owner),
            },
            Member {
                user: 2,
                name: Some("hong".into()),
                rank: None,
            },
        ]
    );
    assert_eq!(member(&json!({"card": "没号"})), None);
}

#[test]
fn a_fetched_message_says_where_it_was() {
    let group = fetched(&json!({"group_id": "888", "user_id": 5, "message": "hi"}));
    assert_eq!(
        (group.private, group.group, group.user),
        (false, Some(888), Some(5))
    );
    assert_eq!(group.segments.pieces.len(), 1);
    let private = fetched(&json!({"message_type": "private", "user_id": 6}));
    assert_eq!(
        (private.private, private.group, private.user),
        (true, None, Some(6))
    );
}

#[test]
fn a_group_name_is_taken_as_is_when_it_is_text() {
    assert_eq!(
        group_name(&json!({"group_name": " 摸鱼群 "})).as_deref(),
        Some(" 摸鱼群 ")
    );
    assert_eq!(group_name(&json!({"group_name": 5})), None);
    assert_eq!(group_name(&json!({})), None);
}
