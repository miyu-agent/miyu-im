//! 群公告的读法（施工 I-37）。

use serde_json::json;

use super::*;

#[test]
fn notices_are_newest_first_and_read_from_several_shapes() {
    let data = json!([
        {"sender_id": 1, "publish_time": 1_700_000_000, "message": {"text": "群规：不刷屏"}},
        {"sender_id": 2, "publish_time": 1_760_000_000, "message": "周六十点东门集合"},
        {"sender_id": 3, "publish_time": 1_750_000_000, "content": "  新人先看群文件  "},
        {"sender_id": 4, "publish_time": 1_790_000_000, "message": {"images": []}},
    ]);
    let read = notices(&data);
    let texts: Vec<&str> = read.iter().map(|one| one.text.as_str()).collect();
    assert_eq!(
        texts,
        ["周六十点东门集合", "新人先看群文件", "群规：不刷屏"],
        "读不出正文的不列"
    );
    assert_eq!(read[0].sender, Some(2));
}

#[test]
fn at_most_ten_and_each_is_cut() {
    let many: Vec<_> = (0..15)
        .map(|n| json!({"sender_id": 1, "publish_time": 1_700_000_000 + n, "message": {"text": "长".repeat(600)}}))
        .collect();
    let read = notices(&json!(many));
    assert_eq!(read.len(), 10);
    assert!(read[0].text.chars().count() <= 500);
    assert_eq!(notices(&json!(null)), Vec::new());
}

#[test]
fn days_are_written_in_utc() {
    assert_eq!(day(1_761_177_600), "2025-10-23");
    assert_eq!(day(0), "?");
    assert_eq!(day(951_782_400), "2000-02-29");
}
