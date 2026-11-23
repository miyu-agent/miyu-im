//! 引用的那一条是谁发的（施工 LW-IM-7）：范围以内的人话认得出；她的、范围以外的、别的场所的、库里没有的认不出。

use super::super::{FILE, Said, Write, prepared, written};
use super::sender_of;

fn said(venue: &str, msg: &str, sender: &str, at: i64, mine: bool) -> Write {
    Write::Said(Box::new(Said {
        venue: venue.to_string(),
        msg: msg.to_string(),
        sender: sender.to_string(),
        name: None,
        at,
        text: "话".to_string(),
        reply_to: None,
        mentions: Vec::new(),
        media: Vec::new(),
        mine,
    }))
}

#[test]
fn only_human_messages_inside_the_range_are_found() {
    let dir = std::env::temp_dir().join(format!("im-records-{}-quoted", std::process::id()));
    if std::fs::remove_dir_all(&dir).is_err() {
        // 没有就是没有。
    }
    let connection = prepared(&dir.join(FILE)).expect("开得了");
    let venue = "qq:30003:group:31";
    for write in [
        said(venue, "1", "qq:7", 1_000, false),
        said(venue, "2", "qq:30003", 2_000, true),
        said(venue, "3", "qq:8", 9_000, false),
        said("qq:30003:group:32", "4", "qq:9", 1_500, false),
    ] {
        written(&connection, &write).expect("记得上");
    }
    let found = |msg: &str, range| sender_of(&connection, venue, msg, range).expect("查得了");
    assert_eq!(found("1", (0, 5_000)), Some("qq:7".to_string()));
    assert_eq!(found("2", (0, 5_000)), None, "她的不算");
    assert_eq!(found("3", (0, 5_000)), None, "范围以外的不算");
    assert_eq!(found("4", (0, 5_000)), None, "别的场所的不算");
    assert_eq!(found("5", (0, 5_000)), None, "库里没有的");
    drop(connection);
    std::fs::remove_dir_all(&dir).expect("删得掉");
}
