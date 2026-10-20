//! 违规关键词（施工 I-16，照旧版设置页能改违规关键词；2026-10-20 项目主人定违规判断的设置上网页）：
//!
//! - `moderation.get` → `{keywords, own}`：系统那份词表（`system/modules/im/moderation.txt`，在的话整份替换出厂的，`rules`）里的词；
//!   没有系统那份的照出厂的，`own` 是假。
//! - `moderation.set {keywords}`：写系统那一份（一行一个，开头一段说明）；`keywords` 是 `null` 的删掉系统那份、回到出厂的。桥隔一会儿
//!   看一眼系统的变没变（`rules::Venues`），变了整份重读，下一条消息照新的。
//!
//! 违规的严重程度门槛、只查违规那一次的超时是场所规则（会话浮窗「高级」），不在这里。

use std::path::Path;

use miyu_chat::Moderation;
use serde_json::{Value, json};

use super::Site;
use super::plugins::bad;

/// 写进系统那份开头的说明。
const HEADER: &str =
    "# 违规关键词（接入通讯平台的网页上改的，整份替换出厂的词表）。一行一个词；# 开头的行不算。\n";

/// 一个词最多几个字符、最多几个词：只防写错。
const WORD: usize = 100;
const MOST: usize = 2000;

/// `moderation.get`。
pub(super) fn get(site: &Site) -> Result<Value, (String, String)> {
    let own = crate::rules::words(&site.root);
    let (text, mine) = match std::fs::read_to_string(&own) {
        Ok(text) => (text, true),
        Err(_) => (factory(&site.files).unwrap_or_default(), false),
    };
    Ok(json!({"keywords": Moderation::parse_keywords(&text), "own": mine}))
}

/// `moderation.set`。
pub(super) fn set(site: &Site, params: &Value) -> Result<Value, (String, String)> {
    let own = crate::rules::words(&site.root);
    if params["keywords"].is_null() {
        return match std::fs::remove_file(&own) {
            Ok(()) => Ok(json!({"saved": true})),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(json!({"saved": true}))
            }
            Err(error) => Err(("write_failed".to_string(), error.to_string())),
        };
    }
    let words = words(&params["keywords"]).ok_or_else(bad)?;
    let mut text = HEADER.to_string();
    for word in &words {
        text.push_str(word);
        text.push('\n');
    }
    write(&own, &text).map_err(|error| ("write_failed".to_string(), error.to_string()))?;
    Ok(json!({"saved": true}))
}

/// 页面交来的词（纯逻辑）：每个是字、去掉首尾空白、不空、不含换行、不以 `#` 开头、不超过 [`WORD`] 个字符，最多 [`MOST`] 个；
/// 重复的只留第一个。有一个不对的整份不收。
pub(super) fn words(value: &Value) -> Option<Vec<String>> {
    let list = value.as_array()?;
    if list.len() > MOST {
        return None;
    }
    let mut words: Vec<String> = Vec::new();
    for one in list {
        let word = one.as_str()?.trim();
        if word.is_empty()
            || word.contains('\n')
            || word.starts_with('#')
            || word.chars().count() > WORD
        {
            return None;
        }
        if !words.iter().any(|known| known == word) {
            words.push(word.to_string());
        }
    }
    Some(words)
}

/// 出厂的词表：页面的文件在包文件夹的 `site/`，词表在旁边的 `data/`。
fn factory(files: &Path) -> Option<String> {
    let dir = files.parent()?;
    std::fs::read_to_string(dir.join("data").join("moderation.txt")).ok()
}

/// 写进 `path`：先写旁边的 `.partial` 再换名。
fn write(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let partial = path.with_extension("txt.partial");
    std::fs::write(&partial, text)?;
    std::fs::rename(&partial, path)
}

#[cfg(test)]
mod tests;
