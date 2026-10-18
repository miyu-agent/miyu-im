//! 两页上给人看的字（`data/cards/human/page.json`）：每个键三种语言并列。

use std::collections::BTreeMap;

/// 两页上给人看的字：`{键: {语言: 字}}`。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Words(pub(super) BTreeMap<String, BTreeMap<String, String>>);

impl Words {
    /// 读 `page.json` 的原文：写法不对的交回空的。
    pub fn parse(text: &str) -> Option<Words> {
        serde_json::from_str(text).ok().map(Words)
    }

    /// 一句话：这种语言没有的照英文，都没有的照键（缺了看得出来）。
    pub fn get(&self, key: &str, lang: &str) -> String {
        self.0
            .get(key)
            .and_then(|by| by.get(lang).or_else(|| by.get("en")))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// 有没有这一句。
    pub fn has(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }
}
