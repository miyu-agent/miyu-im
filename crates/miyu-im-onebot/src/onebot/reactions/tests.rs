//! QQ 能贴的表情的列表：编号只写十进制、不重，Unicode 表情的编号是那个字的码点，出厂的排第一个。

use super::REACTIONS;

#[test]
fn ids_are_decimal_numbers_and_glyphs_match_their_code_point() {
    // 带 0x 的十六进制手机 QQ 认不出（画成空白方块），编号只写十进制；Unicode 表情的编号就是那个字的码点。
    let mut seen = std::collections::BTreeSet::new();
    for one in REACTIONS {
        assert!(one.id.bytes().all(|byte| byte.is_ascii_digit()), "{one:?}");
        assert!(seen.insert(one.id), "编号重了：{one:?}");
        if let Some(glyph) = one.glyph {
            let point = glyph.chars().next().map(u32::from).expect("有字");
            assert_eq!(one.id, point.to_string(), "{one:?}");
        }
    }
    assert_eq!(REACTIONS[0].id, "289", "出厂贴的排第一个");
}
