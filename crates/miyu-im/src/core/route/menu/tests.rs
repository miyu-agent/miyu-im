use serde_json::json;

use super::{items, owners};
use crate::chat::{Id, MenuItem, Platform};

#[test]
fn owners_are_the_bindings_of_this_platform_with_a_value() {
    let config = json!({"items": {
        "external.bindings.\"tg:7581443271\"": {"value": "owner"},
        "external.bindings.\"qq:10001\"": {"value": "owner"},
        "external.bindings.\"tg:42\"": {"value": ""},
        "external.bindings.tg:9": {"value": "admin"},
        "im.qq.listen": {"value": 8301},
    }});
    let mut found = owners(&config, Platform::new("tg"));
    found.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    assert_eq!(
        found,
        [
            Id::new("7581443271").expect("号"),
            Id::new("9").expect("号")
        ]
    );
    assert_eq!(
        owners(&config, Platform::new("qq")),
        [Id::new("10001").expect("号")]
    );
}

#[test]
fn the_catalog_reads_as_names_and_summaries() {
    let catalog = json!({"commands": [
        {"name": "clear-memory", "aliases": [], "summary": "清掉记忆"},
        {"name": "help", "aliases": []},
        {"aliases": []},
    ]});
    assert_eq!(
        items(&catalog),
        [
            MenuItem {
                name: "clear-memory".to_string(),
                summary: "清掉记忆".to_string()
            },
            MenuItem {
                name: "help".to_string(),
                summary: String::new()
            },
        ]
    );
}
