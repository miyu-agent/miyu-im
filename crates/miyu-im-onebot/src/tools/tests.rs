//! 驱动交的工具和出厂清单的功能表对得上（施工 I-91）：清单写了几个功能，登记的工具哪个功能都没列的，核心整批不收（核心 T-2）。

use std::collections::BTreeMap;

use super::{LOOKUPS, TOOLS};

/// 出厂清单 `[features]` 里每个功能列的工具。
fn features() -> BTreeMap<String, Vec<String>> {
    let manifest: toml::Table =
        toml::from_str(include_str!("../../../../miyu-im/package.toml")).expect("清单是 TOML");
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
fn the_lookup_feature_is_the_lookup_plugin() {
    // 预设里的「查询信息」和场所插件「查询信息」管的是同一批（施工 I-91）。
    let mut feature = features().remove("im-lookup").expect("有查询信息");
    let mut plugin: Vec<String> = LOOKUPS.iter().map(|tool| (*tool).to_string()).collect();
    feature.sort();
    plugin.sort();
    assert_eq!(feature, plugin);
}
