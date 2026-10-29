use serde_json::json;

use super::*;

#[test]
fn a_connection_holds_its_platforms_venues_and_its_own_bots() {
    let scope = Scope::of(&json!({"connection": "qq:10001"}))
        .expect("写法对")
        .expect("选了一个");
    // I-81 以前的编号不带号，照平台算；带号的认号。
    assert!(within(Some(&scope), "qq:group:777"));
    assert!(within(Some(&scope), "qq:10001:group:777"));
    assert!(!within(Some(&scope), "qq:10002:group:777"));
    assert!(!within(Some(&scope), "tg:group:-100"));
    assert!(within(None, "tg:group:-100"), "「全部」什么都在");
}

#[test]
fn a_connection_is_written_platform_colon_bot() {
    assert!(Scope::of(&json!({})).expect("没写是全部").is_none());
    for wrong in [
        json!("qq"),
        json!("qq:"),
        json!(":1"),
        json!("qq:1:2"),
        json!(7),
    ] {
        assert!(Scope::of(&json!({"connection": wrong})).is_err(), "{wrong}");
    }
}
