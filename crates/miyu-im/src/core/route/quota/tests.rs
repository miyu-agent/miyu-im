//! 判额度：核心 `usage.query` 照场所分的几行读成场所 → 四项合计，没有场所的不要。

use serde_json::json;

use super::rows;

#[test]
fn rows_sum_the_four_kinds_per_venue_and_skip_rows_without_a_venue() {
    let used = rows(&json!([
        {"venue": "qq:1:group:5", "usage": {"uncached": 1, "cache_read": 20, "cache_write": 300, "output": 4000}},
        {"venue": null, "usage": {"uncached": 9, "cache_read": 9, "cache_write": 9, "output": 9}},
        {"venue": "qq:1:private:7", "usage": {"uncached": 5}},
    ]));
    assert_eq!(used.len(), 2);
    assert_eq!(used["qq:1:group:5"], 4321);
    assert_eq!(used["qq:1:private:7"], 5, "缺的几项当 0");
}
