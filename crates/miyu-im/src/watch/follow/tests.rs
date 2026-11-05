//! 命令行认会话（施工 I-160 下）。

use super::{Unresolved, resolve};

fn seen() -> Vec<(String, String)> {
    [
        ("qq:group:777", "验收测试群"),
        ("qq:private:20004", "小林"),
        ("qq:group:5", "群五"),
        ("qq:private:5", "五号"),
    ]
    .into_iter()
    .map(|(venue, name)| (venue.to_string(), name.to_string()))
    .collect()
}

#[test]
fn a_bare_number_is_a_group_or_a_private_chat_seen_before() {
    assert_eq!(resolve("qq:777", &seen()), Ok("qq:group:777".to_string()));
    assert_eq!(
        resolve("qq:20004", &seen()),
        Ok("qq:private:20004".to_string())
    );
    assert_eq!(
        resolve("qq:5", &seen()),
        Err(Unresolved::Both("5".to_string())),
        "两边都见过的要写全"
    );
    assert_eq!(resolve("qq:6", &seen()), Err(Unresolved::Unknown));
}

#[test]
fn a_full_venue_is_taken_as_written() {
    assert_eq!(
        resolve("qq:group:6", &seen()),
        Ok("qq:group:6".to_string()),
        "写全的没见过也照它看"
    );
    assert_eq!(
        resolve("qq:private:5", &seen()),
        Ok("qq:private:5".to_string())
    );
    assert_eq!(
        resolve("777", &seen()),
        Err(Unresolved::Unknown),
        "不带平台的认不出"
    );
}
