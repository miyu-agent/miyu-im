//! 配置项那一层（`onebot.md` 第一条「场所规则和出厂数据」第 8 条，施工 O-41）：群白名单、白名单以外的群开不开、两档限流、
//! 睡觉时间、群里的触发词、看不看得到 QQ 号这七项是包的配置项，网页、终端的设置页照清单自己画。桥照它们拼一份规则文件
//! [`FILE`]，来处是 [`Source::Settings`]，和出厂的、系统的一起交群聊内核读：写法、怎么套、写错了丢哪一项都照规则文件的规矩。
//!
//! - 一项一条规则，先后固定（[`ORDER`]）：第几条规则就是哪一项，问题、来处照它说成配置项的键（[`key_of`]）。没设、等于默认
//!   的那一项，规则照写、只有 `match`，什么都不设。
//! - 白名单群那两条在最后，盖掉对全部群的几条。群白名单是空的，那两条的 `match` 是空的群号列表，不匹配任何群。
//! - 字照 JSON 的写法写进去：JSON 的转义（`\"`、`\\`、`\n`、`\u0001` 这些）都是 TOML 基本字符串认的，不另加依赖。

use serde_json::{Map, Value};

use miyu_chat::{File, Source};

use crate::PACKAGE;

/// 拼出来的那一份的文件名：排在出厂的 `50-defaults.toml` 后面；系统里名字排在它后面的盖它，同名的整份替换它。
pub const FILE: &str = "60-settings.toml";

/// 七项在清单里的名字，照拼出来的规则的先后：第 n 条规则是第 n 项。
const ORDER: [&str; 7] = [
    OTHER_GROUP_RATE,
    OTHER_GROUPS,
    KEYWORDS,
    SLEEP,
    SHOW_IDS,
    GROUPS,
    GROUP_RATE,
];

/// 群白名单：群号的列表。
const GROUPS: &str = "groups";

/// 白名单以外的群也接话：开关，默认开。
const OTHER_GROUPS: &str = "other_groups";

/// 白名单群的限流：写法同规则的 `rate`。
const GROUP_RATE: &str = "group_rate";

/// 其他群的限流：写法同规则的 `rate`。
const OTHER_GROUP_RATE: &str = "other_group_rate";

/// 睡觉时间：写法同规则的 `sleep`，群和私聊都算。
const SLEEP: &str = "sleep";

/// 群里的触发词：字的列表。
const KEYWORDS: &str = "keywords";

/// 她看得到发话人的 QQ 号：开关，默认关。
const SHOW_IDS: &str = "show_ids";

/// 七项这一刻的值：握手交来的，推来变了的照换。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Configured {
    /// 群白名单。
    groups: Vec<u64>,
    /// 白名单以外的群也接话。
    other_groups: bool,
    /// 白名单群的限流，原文；没设的是空的。
    group_rate: Option<String>,
    /// 其他群的限流，原文；没设的是空的。
    other_group_rate: Option<String>,
    /// 睡觉时间，原文；没设的是空的。
    sleep: Option<String>,
    /// 群里的触发词。
    keywords: Vec<String>,
    /// 她看得到 QQ 号。
    show_ids: bool,
}

impl Default for Configured {
    /// 七项都没设：照清单的默认值，白名单以外的群照样接话。
    fn default() -> Configured {
        Configured {
            groups: Vec::new(),
            other_groups: true,
            group_rate: None,
            other_group_rate: None,
            sleep: None,
            keywords: Vec::new(),
            show_ids: false,
        }
    }
}

impl Configured {
    /// 握手交来的 `config`（`venue show` 照 `config.get` 的值拼成同样的样子）：没交的照默认。不是对象的当什么都没交。
    pub fn handed(config: &Value) -> Configured {
        let mut configured = Configured::default();
        if let Some(config) = config.as_object() {
            configured.change(config);
        }
        configured
    }

    /// 推来的变化 `keys` 合进来：带了的键照新值，`null`、类型不对的当没设（核心照清单查过类型，照说不会）；没带的不动，别的
    /// 键不认。交回七项变了没有。
    pub fn change(&mut self, keys: &Map<String, Value>) -> bool {
        let before = self.clone();
        let fallback = Configured::default();
        for (key, value) in keys {
            let Some(name) = key
                .strip_prefix(PACKAGE)
                .and_then(|rest| rest.strip_prefix('.'))
            else {
                continue;
            };
            match name {
                GROUPS => self.groups = numbers(value),
                OTHER_GROUPS => {
                    self.other_groups = value.as_bool().unwrap_or(fallback.other_groups)
                }
                GROUP_RATE => self.group_rate = text(value),
                OTHER_GROUP_RATE => self.other_group_rate = text(value),
                SLEEP => self.sleep = text(value),
                KEYWORDS => self.keywords = texts(value),
                SHOW_IDS => self.show_ids = value.as_bool().unwrap_or(fallback.show_ids),
                _ => {}
            }
        }
        *self != before
    }

    /// 拼成规则文件（来处 [`Source::Settings`]，名字 `60-settings.toml`），七条规则照固定的先后（[`key_of`]）。
    pub fn file(&self) -> File {
        let all_groups = "{ kind = \"group\" }".to_string();
        let whitelisted = format!(
            "{{ kind = \"group\", group = [{}] }}",
            self.groups
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
        let mut text = String::new();
        for name in ORDER {
            let (matcher, attribute) = match name {
                OTHER_GROUP_RATE => (
                    Some(&all_groups),
                    self.other_group_rate
                        .as_deref()
                        .map(|rate| ("rate", quoted(rate))),
                ),
                OTHER_GROUPS => (
                    Some(&all_groups),
                    (!self.other_groups).then(|| ("allow", "false".to_string())),
                ),
                KEYWORDS => (
                    Some(&all_groups),
                    (!self.keywords.is_empty()).then(|| {
                        let words: Vec<String> =
                            self.keywords.iter().map(|word| quoted(word)).collect();
                        ("keywords", format!("[{}]", words.join(", ")))
                    }),
                ),
                SLEEP => (
                    None,
                    self.sleep.as_deref().map(|sleep| ("sleep", quoted(sleep))),
                ),
                SHOW_IDS => (
                    None,
                    self.show_ids.then(|| ("show_ids", "true".to_string())),
                ),
                GROUPS => (
                    Some(&whitelisted),
                    (!self.groups.is_empty()).then(|| ("allow", "true".to_string())),
                ),
                _ => (
                    Some(&whitelisted),
                    self.group_rate
                        .as_deref()
                        .map(|rate| ("rate", quoted(rate))),
                ),
            };
            text.push_str("[[rule]]\n");
            if let Some(matcher) = matcher {
                text.push_str(&format!("match = {matcher}\n"));
            }
            if let Some((key, value)) = attribute {
                text.push_str(&format!("{key} = {value}\n"));
            }
            text.push('\n');
        }
        File {
            source: Source::Settings,
            name: FILE.to_string(),
            text,
        }
    }
}

/// 拼出来的第 `rule` 条规则（从 1 数）是哪一项的：配置项的键（`onebot.group_rate`）；不在七条里的是空的。
pub fn key_of(rule: usize) -> Option<String> {
    rule.checked_sub(1)
        .and_then(|index| ORDER.get(index))
        .map(|name| format!("{PACKAGE}.{name}"))
}

/// 照 JSON 的写法加上引号：JSON 的转义都是 TOML 基本字符串认的。
fn quoted(text: &str) -> String {
    Value::from(text).to_string()
}

/// 一段字：空的、不是字的当没设。
fn text(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

/// 字的列表：不是列表的是空的，里面不是字的不要。
fn texts(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// 群号的列表：不是列表的是空的，里面不是非负整数的不要。
fn numbers(value: &Value) -> Vec<u64> {
    value
        .as_array()
        .map(|list| list.iter().filter_map(Value::as_u64).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
