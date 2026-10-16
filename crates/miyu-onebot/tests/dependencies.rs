//! 依赖的方向（这个仓库没有核心那样的分层门禁，由这里守）：桥不依赖核心的 crate（施工 O-20，`onebot.md` 第一条「施工时定的」
//! 第 6、48 条；`18-通讯平台.md` 第一节），配置由核心在握手时交、变了推过来；核心的东西只经门面 `miyu-client` 拿，外加群聊
//! 内核（设计 32 第九节第 1 条，施工 S-6 上）。群聊内核在第 2 层，只依赖内核、配置（`chat.md`「是什么」，施工 O-40）。照
//! `cargo metadata` 看；开发依赖只给测试用，不算。

use std::process::Command;

use serde_json::Value;

/// 桥不许依赖的：核心的两个 crate。
const CORE: [&str; 2] = ["miyu-core", "miyu-endpoint"];

#[test]
fn the_bridge_does_not_depend_on_the_core_crates() {
    let used = used("miyu-onebot");
    assert!(
        used.contains(&"miyu-client".to_string()),
        "读到了依赖：{used:?}"
    );
    for core in CORE {
        assert!(!used.contains(&core.to_string()), "依赖了 {core}：{used:?}");
    }
    assert_eq!(
        ours(&used),
        ["miyu-chat", "miyu-client"],
        "只经门面拿核心的东西"
    );
}

#[test]
fn the_chat_kernel_depends_only_on_the_kernel_and_config() {
    let used = used("miyu-chat");
    assert_eq!(
        ours(&used),
        ["miyu-config", "miyu-kernel"],
        "第 2 层不用门面和更高层"
    );
}

/// 工作区里 `package` 这个 crate 的正式依赖的名字（开发依赖只给测试用，不算）。
fn used(package: &str) -> Vec<String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .args(["--manifest-path", manifest])
        .output()
        .expect("跑得了 cargo metadata");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).expect("是 JSON");
    let found = metadata["packages"]
        .as_array()
        .expect("有 packages")
        .iter()
        .find(|found| found["name"] == package)
        .unwrap_or_else(|| panic!("有 {package}"));
    found["dependencies"]
        .as_array()
        .expect("有 dependencies")
        .iter()
        .filter(|dependency| dependency["kind"] != "dev")
        .filter_map(|dependency| dependency["name"].as_str())
        .map(str::to_string)
        .collect()
}

/// 依赖里 Miyu 自己的那几个（`miyu-` 开头），排好。
fn ours(used: &[String]) -> Vec<&str> {
    let mut ours: Vec<&str> = used
        .iter()
        .map(String::as_str)
        .filter(|name| name.starts_with("miyu-"))
        .collect();
    ours.sort_unstable();
    ours
}
