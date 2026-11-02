//! 私聊默认值（施工 I-112，项目主人 11-02：会话默认值只有群聊的格，私聊白名单的卡上没有的也管不了；拆成群聊、私聊两张）。
//!
//! - 写过私聊默认值的照它拼一份，只配私聊。
//! - 没写过的（I-112 以前的 `cards.json`）：私聊照旧照群聊默认值（原来的「会话默认值」）拼的那几行套，意思不变，不用搬；页面上画
//!   私聊默认值那张卡时照它里面私聊卡也有的格（[`Cards::private_set`]），存第一格时照这一份抄过来再改。

use miyu_chat::File;

use super::lines::{Matcher, defaults_lines, file, private_lines};
use super::{Card, Cards, GroupSet, PrivateSet};

impl Cards {
    /// 私聊默认值：写过的照它；没写过的照群聊默认值里私聊卡也有的格（`from_group`）。
    pub fn private_set(&self) -> PrivateSet {
        self.private_defaults
            .clone()
            .unwrap_or_else(|| from_group(&self.defaults))
    }

    /// 私聊默认值一格都没写（没写过的照群聊默认值算）。
    pub(super) fn private_defaults_unset(&self) -> bool {
        match &self.private_defaults {
            Some(set) => *set == PrivateSet::default(),
            None => self.defaults == GroupSet::default(),
        }
    }

    /// 私聊默认值拼成的那一份，只配私聊：写过的照它；没写过的照群聊默认值的那几行（同 I-112 以前，私聊照旧）。
    pub fn private_defaults_file(&self) -> File {
        let lines = match &self.private_defaults {
            Some(set) => private_lines(set),
            None => defaults_lines(&self.defaults),
        };
        file(Card::PrivateDefaults, &Matcher::Own, &lines)
    }
}

/// 群聊默认值 `group` 里私聊卡也有的格；限流、睡觉时间私聊卡写在 `rules` 里（`rate`、`sleep`），搬过去。
fn from_group(group: &GroupSet) -> PrivateSet {
    let mut rules = group.rules.clone().unwrap_or_default();
    if let Some(rate) = &group.rate {
        rules
            .entry("rate".to_string())
            .or_insert_with(|| serde_json::json!(rate));
    }
    if let Some(sleep) = &group.sleep {
        rules
            .entry("sleep".to_string())
            .or_insert_with(|| serde_json::json!(sleep));
    }
    PrivateSet {
        show_ids: group.show_ids,
        persona: group.persona.clone(),
        model: group.model.clone(),
        preset: group.preset.clone(),
        extra_prompt: group.extra_prompt.clone(),
        long_text: group.long_text,
        long_chars: group.long_chars,
        final_only: group.final_only,
        glance: group.glance,
        plugins: group.plugins.clone(),
        rules: (!rules.is_empty()).then_some(rules),
    }
}

#[cfg(test)]
mod tests;
