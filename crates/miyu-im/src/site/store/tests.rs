//! 商城只列 im 的插件（施工 I-11）。

use serde_json::json;

use super::plugins;

#[test]
fn only_packages_carrying_im_content_are_listed() {
    let found = json!([
        {"package": "miyu-im-memes", "contents": ["im"], "official": true},
        {"package": "miyu-embed", "contents": [], "official": true},
        {"package": "miyu-web", "official": true},
        {"package": "both", "contents": ["skills", "im"], "official": false},
    ]);
    let listed: Vec<String> = plugins(&found)
        .iter()
        .map(|item| item["package"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(listed, ["miyu-im-memes", "both"]);
    assert!(plugins(&json!(null)).is_empty());
}
