//! `chat.note` 的参数（施工 I-8 上）：种类当标签名，只收小写字母开头、小写字母数字和 `-`、最多 32 个字；字去掉首尾空白不空、最多
//! 500 个字；场所照核心的写法。

use serde_json::json;

use super::{kind_ok, read};

#[test]
fn a_kind_is_a_safe_tag_name() {
    for kind in ["moderated", "a", "join-request", "x2"] {
        assert!(kind_ok(kind), "{kind}");
    }
    for kind in [
        "",
        "Moderated",
        "2x",
        "-a",
        "a b",
        "a_b",
        "a>",
        "禁言",
        &"a".repeat(33),
    ] {
        assert!(!kind_ok(kind), "{kind}");
    }
    assert!(kind_ok(&"a".repeat(32)));
}

#[test]
fn a_note_reads_its_venue_kind_and_trimmed_text() {
    let args = json!({"venue": "qq:group:777", "kind": "moderated", "text": "  小周 was muted.
"});
    let (venue, kind, text) = read(&args).expect("读得出");
    assert_eq!(
        (venue.id().as_str(), kind, text),
        ("qq:group:777", "moderated", "小周 was muted.")
    );
    for wrong in [
        json!({"venue": "qq:group:777", "kind": "moderated", "text": "  "}),
        json!({"venue": "qq:group:777", "kind": "moderated", "text": "字".repeat(501)}),
        json!({"venue": "qq:group:777", "kind": "Moderated", "text": "x"}),
        json!({"venue": "not a venue", "kind": "moderated", "text": "x"}),
        json!({"kind": "moderated", "text": "x"}),
    ] {
        assert!(read(&wrong).is_none(), "{wrong}");
    }
    assert!(
        read(&json!({"venue": "qq:group:777", "kind": "a", "text": "字".repeat(500)})).is_some()
    );
}
