//! 驱动交的工具和出厂清单的功能表对得上（同 OneBot 的，施工 I-91、I-183 下下）：登记的工具哪个功能都没列的，核心整批不收（核心
//! T-2），连 QQ 的也一起登记不上。清单里也列着 QQ 的：这里只核对 `tg_` 开头的。

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use super::{TOOLS, connected};

/// 出厂清单 `[features]` 里每个功能列的 Telegram 的工具。
fn features() -> BTreeMap<String, Vec<String>> {
    let manifest: toml::Table =
        toml::from_str(include_str!("../../../../im/package.toml")).expect("清单是 TOML");
    manifest["features"]
        .as_table()
        .expect("有功能表")
        .iter()
        .map(|(id, feature)| {
            let tools = feature["tools"]
                .as_array()
                .expect("功能列了工具")
                .iter()
                .map(|tool| tool.as_str().expect("工具名是字").to_string())
                .filter(|tool| tool.starts_with("tg_"))
                .collect();
            (id.clone(), tools)
        })
        .collect()
}

#[test]
fn every_tool_belongs_to_exactly_one_feature() {
    let features = features();
    for tool in TOOLS.iter().map(|decl| decl.name) {
        let owners: Vec<&String> = features
            .iter()
            .filter(|(_, tools)| tools.iter().any(|one| one == tool))
            .map(|(id, _)| id)
            .collect();
        assert_eq!(owners.len(), 1, "{tool} 归 {owners:?}");
    }
    let listed: usize = features.values().map(Vec::len).sum();
    assert_eq!(listed, TOOLS.len(), "清单没有多列驱动不交的工具");
}

#[test]
fn the_tools_are_offered_only_with_a_token() {
    let config = |tokens: Value| -> Map<String, Value> {
        json!({"tokens": tokens})
            .as_object()
            .cloned()
            .expect("是表")
    };
    assert!(!connected(&Map::new()), "没写连接表");
    assert!(!connected(&config(json!([]))), "连接表空的");
    assert!(!connected(&config(json!([null, ""]))), "密钥没取到的、空的");
    assert!(connected(&config(json!([null, "111:abc"]))));
    assert!(
        connected(&config(json!([{"secret": "tg"}]))),
        "还是引用的也算写了"
    );
}
