//! 交给页面的插件（`super`）：写了的链接、图标、平台、能力都带上，没写的不带；用不了的说为什么，缺能力的说缺哪几样；读不成的只有编号。

use std::collections::BTreeMap;

use serde_json::json;

use super::*;
use crate::chat::CAPS;
use crate::plugins::declared;

#[test]
fn written_details_go_out_and_unwritten_ones_stay_out() {
    let full = "[plugin]\nname = { zh = \"群管\" }\nsummary = { en = \"Admin\" }\nicon = \"shield\"\nhomepage = \"https://a.invalid\"\nreadme = \"README.md\"\nplatforms = [\"qq\"]\nneeds = [\"mute\"]\n";
    let plugins = Plugins::new(BTreeMap::from([
        (
            "bare".to_string(),
            declared::read("[plugin]\nname = { en = \"x\" }\n"),
        ),
        (
            "broken".to_string(),
            Err("plugin.toml: not_found".to_string()),
        ),
        ("full".to_string(), declared::read(full)),
        (
            "wants".to_string(),
            declared::read("[plugin]\nname = { en = \"y\" }\nneeds = [\"forward\", \"mute\"]\n"),
        ),
    ]));
    let listed = listed(&plugins, Platform::new("qq"), &[Cap::Mute]);
    assert_eq!(
        listed[0],
        json!({"package": "bare", "name": {"en": "x"}, "summary": {}, "default": true})
    );
    assert_eq!(
        listed[1],
        json!({"package": "broken", "unusable": "broken"})
    );
    assert_eq!(
        listed[2],
        json!({
            "package": "full", "name": {"zh": "群管"}, "summary": {"en": "Admin"}, "default": true,
            "icon": "shield", "links": {"homepage": "https://a.invalid", "readme": "README.md"},
            "platforms": ["qq"], "needs": ["mute"],
        })
    );
    assert_eq!(listed[3]["unusable"], "missing");
    assert_eq!(listed[3]["missing"], json!(["forward"]));
    let elsewhere = listed_on(&plugins, "tg");
    assert_eq!(elsewhere[2]["unusable"], "other_platforms");
    assert!(elsewhere[2].get("missing").is_none());
}

/// 在平台 `name` 上、能力齐全时交出去的。
fn listed_on(plugins: &Plugins, name: &'static str) -> Vec<serde_json::Value> {
    listed(plugins, Platform::new(name), &CAPS)
}
