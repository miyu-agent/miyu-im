use serde_json::json;

use super::words;

#[test]
fn keywords_from_the_page_are_cleaned_or_refused() {
    assert_eq!(
        words(&json!([" 赌博 ", "代开发票", "赌博"])),
        Some(vec!["赌博".to_string(), "代开发票".to_string()])
    );
    assert_eq!(words(&json!([])), Some(Vec::new()));
    for bad in [
        json!(["# 注释"]),
        json!([""]),
        json!(["a\nb"]),
        json!([1]),
        json!("赌博"),
    ] {
        assert_eq!(words(&bad), None, "{bad}");
    }
}
