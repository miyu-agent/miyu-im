//! 参数的读法（施工 I-17）：号写成数字的字也认，列表写成 JSON 字的也认；写错的说哪个参数。

use serde_json::json;

use super::*;

#[test]
fn a_number_written_as_text_is_read() {
    assert_eq!(qq_number(&json!(12345)), Some(12345));
    assert_eq!(qq_number(&json!(" 12345 ")), Some(12345));
    for bad in [
        json!(0),
        json!(-3),
        json!("小明"),
        json!(1.5),
        json!(true),
        json!(null),
    ] {
        assert_eq!(qq_number(&bad), None, "{bad}");
    }
    let args = json!({"user": "40001", "group": "群"});
    assert_eq!(optional_number(&args, "user", "u"), Ok(Some(40001)));
    assert_eq!(optional_number(&args, "nobody", "n"), Ok(None));
    assert_eq!(
        optional_number(&args, "group", "group must be a number"),
        Err("group must be a number")
    );
}

#[test]
fn a_list_is_deduplicated_and_kept_within_bounds() {
    assert_eq!(
        qq_numbers(&json!([40001, "40002", 40001]), 32),
        Some(vec![Id::from(40001), Id::from(40002)])
    );
    assert_eq!(
        qq_numbers(&json!("[40001, 40002]"), 32),
        Some(vec![Id::from(40001), Id::from(40002)]),
        "写成字的数组"
    );
    assert_eq!(qq_numbers(&json!([]), 32), None);
    assert_eq!(
        qq_numbers(&json!([40001, "x"]), 32),
        None,
        "有写错的整个不收"
    );
    assert_eq!(qq_numbers(&json!(40001), 32), None);
    let many: Vec<i64> = (1..=33).collect();
    assert_eq!(qq_numbers(&json!(many), 32), None);
}

#[test]
fn text_must_be_text() {
    let args = json!({"kind": 3, "query": "a"});
    assert_eq!(optional_text(&args, "query", "q"), Ok(Some("a")));
    assert_eq!(optional_text(&args, "none", "q"), Ok(None));
    assert_eq!(
        optional_text(&args, "kind", "kind must be text"),
        Err("kind must be text")
    );
}
