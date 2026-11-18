//! 转发里一条的正文怎么截（施工 O-34，核心审时建议；施工 I-253 起不再先截字数）：只夹到核心的 4096 字节，截在字的边界上。

use super::{TEXT_BYTES, clip};

#[test]
fn text_is_kept_whole_and_only_held_to_the_core_bytes() {
    // 500 个字以上的照原样（原来截到 500 个字）。
    assert_eq!(clip("长".repeat(600).as_str()), "长".repeat(600));
    // 全是 4 字节的字：夹到 1024 个字、正好 4096 字节。
    let cut = clip(&"😀".repeat(2000));
    assert_eq!(cut, "😀".repeat(1024));
    assert_eq!(cut.len(), TEXT_BYTES);
    // 边界不齐的：一个 1 字节的字在前，4 字节的放不下半个，停在 4093 字节。
    let cut = clip(&format!("a{}", "😀".repeat(2000)));
    assert_eq!(cut.len(), 1 + 4 * 1023);
    assert!(cut.ends_with('😀'));
}
