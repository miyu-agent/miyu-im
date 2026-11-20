//! 工具的参数写成各家都收的样子（施工 I-268，2026-11-20 主人用 Antigravity 的 Gemini 撞上）：Gemini 只许字符串带 `enum`，
//! `qq_activity_ranking` 的 `"days":{"type":"integer","enum":[1,7,30]}` 让整个请求被拒（`INVALID_ARGUMENT`），一件工具也用不了。
//! 这里扫 im 自己带的、插件带的全部工具说明：带 `enum` 的只许是 `string`。

use std::path::Path;

use serde_json::Value;

/// 工具说明所在的目录（相对仓库根）。
const DIRS: [&str; 3] = [
    "im/data/onebot/tools",
    "im/data/telegram/tools",
    "im-sponsor/data/tools",
];

#[test]
fn only_strings_carry_an_enum_in_tool_parameters() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    let mut read = 0;
    for dir in DIRS {
        for entry in std::fs::read_dir(root.join(dir)).expect("读得了工具说明的目录") {
            let path = entry.expect("读得了目录项").path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("读得了说明");
            let tool: Value = serde_json::from_str(&text).expect("说明是 JSON");
            read += 1;
            walk(&tool["parameters"], &path.display().to_string(), &mut found);
        }
    }
    assert!(read > 20, "只读到 {read} 份说明");
    assert!(found.is_empty(), "不是字符串却带了 enum：{found:?}");
}

/// 在 `schema` 里找带 `enum` 却不是 `string` 的，记成 `<文件>：<那一段>`。
fn walk(schema: &Value, file: &str, found: &mut Vec<String>) {
    match schema {
        Value::Object(map) => {
            if map.contains_key("enum") && map.get("type") != Some(&Value::from("string")) {
                found.push(format!("{file}：{schema}"));
            }
            for value in map.values() {
                walk(value, file, found);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk(item, file, found);
            }
        }
        _ => {}
    }
}
