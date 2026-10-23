//! 现取带的东西：那一条是不是这个场所的、字节的媒体类型。

use super::*;

fn got(private: bool, group: Option<i64>, user: Option<i64>) -> Fetched {
    Fetched {
        private,
        group,
        user,
        segments: crate::chat::Segments::default(),
    }
}

#[test]
fn only_messages_of_this_venue_are_fetched() {
    assert!(owned(&got(false, Some(31), Some(7)), To::Group(31), 100));
    assert!(
        !owned(&got(false, Some(32), Some(7)), To::Group(31), 100),
        "别的群的"
    );
    assert!(
        owned(&got(true, None, Some(7)), To::Private(7), 100),
        "对方发的"
    );
    assert!(
        owned(&got(true, None, Some(100)), To::Private(7), 100),
        "她发给对方的"
    );
    assert!(
        !owned(&got(true, None, Some(8)), To::Private(7), 100),
        "别人的私聊"
    );
    assert!(
        !owned(&got(false, Some(31), Some(7)), To::Private(7), 100),
        "群里的不算私聊"
    );
}

#[test]
fn image_bytes_get_their_type_and_others_are_octets() {
    assert_eq!(media_type(b"\x89PNG\r\n\x1a\nrest"), "image/png");
    assert_eq!(media_type(b"\xff\xd8\xff\xe0rest"), "image/jpeg");
    assert_eq!(media_type(b"GIF89arest"), "image/gif");
    assert_eq!(media_type(b"%PDF-1.7"), "application/octet-stream");
}
