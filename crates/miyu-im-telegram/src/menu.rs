//! 命令菜单（施工 I-183 下上，项目主人 11-09：私聊里打 `/` 列出 Miyu 的命令，照 Bot API 的 `setMyCommands`）：Miyu 的命令名照
//! Telegram 的规矩改写——只许小写字母、数字、`_`，1 到 32 个字（`-` 换成 `_`，别的不合的不登记）；说明 1 到 256 个字（没有说明的
//! 照名字）；最多 100 条。交回登记的那几条和「登记的名字 → Miyu 的命令名」的对照（收到时换回来）。

use std::collections::HashMap;

use serde_json::{Value, json};

use miyu_im::chat::MenuItem;

/// 菜单最多几条（Bot API）。
const MOST: usize = 100;

/// 照 Miyu 的命令 `items` 拼菜单：登记的几条、对照。
pub fn build(items: &[MenuItem]) -> (Vec<Value>, HashMap<String, String>) {
    let mut commands = Vec::new();
    let mut names = HashMap::new();
    for item in items {
        let name = item.name.to_lowercase().replace('-', "_");
        let fits = (1..=32).contains(&name.len())
            && name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        if !fits || names.contains_key(&name) || commands.len() >= MOST {
            continue;
        }
        let summary = item.summary.trim();
        let summary = if summary.is_empty() {
            name.as_str()
        } else {
            summary
        };
        let description: String = summary.chars().take(256).collect();
        commands.push(json!({"command": name, "description": description}));
        names.insert(name, item.name.clone());
    }
    (commands, names)
}

#[cfg(test)]
mod tests;
