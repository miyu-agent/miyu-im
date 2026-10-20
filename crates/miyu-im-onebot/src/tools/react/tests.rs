//! 贴表情的写法（施工 I-12）。

use super::*;

#[test]
fn an_emoji_becomes_its_code_point_and_numbers_stay_face_ids() {
    assert_eq!(emoji_id("👍"), Some("128077".to_string()));
    assert_eq!(emoji_id(" ❤️ "), Some("10084".to_string()), "头一个字");
    assert_eq!(emoji_id("289"), Some("289".to_string()), "系统表情编号");
    assert_eq!(emoji_id("  "), None);
}
