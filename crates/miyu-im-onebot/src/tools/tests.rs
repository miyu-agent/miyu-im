//! 驱动交的工具和出厂清单的功能表对得上（施工 I-91）：清单写了几个功能，登记的工具哪个功能都没列的，核心整批不收（核心 T-2）。
//! 清单里也列着 Telegram 的（施工 I-183 下下）：这里只核对 `qq_` 开头的。

use std::collections::BTreeMap;

use super::TOOLS;

/// 出厂清单 `[features]` 里每个功能列的 QQ 的工具。
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
                .filter(|tool| tool.starts_with("qq_"))
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
fn features_offered_to_the_terminal_hold_its_tools_and_the_others_hold_none() {
    // 设计 13：只在群、私聊里用的两个功能一件带 `local` 的都没有（核心 PS-2 起预设菜单不列）；带 `local` 的都在给终端的那几个功能里。
    let features = features();
    let local: Vec<&str> = TOOLS
        .iter()
        .filter(|decl| decl.venues.contains(&"local"))
        .map(|decl| decl.name)
        .collect();
    for id in ["im-chat", "im-admin"] {
        let tools = &features[id];
        assert!(
            tools.iter().all(|tool| !local.contains(&tool.as_str())),
            "{id}: {tools:?}"
        );
    }
    for tool in &local {
        let owner = features
            .iter()
            .find(|(_, tools)| tools.iter().any(|one| one == tool))
            .map(|(id, _)| id.as_str());
        assert!(
            matches!(
                owner,
                Some("im-send" | "im-view" | "im-files" | "im-modlog" | "im-usage")
            ),
            "{tool} 归 {owner:?}"
        );
    }
    // 查看聊天那一组都给终端了（设计 13 第一节第 2 条）。
    for tool in &features["im-view"] {
        assert!(local.contains(&tool.as_str()), "{tool}");
    }
}
