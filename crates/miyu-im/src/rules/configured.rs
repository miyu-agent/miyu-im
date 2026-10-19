//! 配置项那一层（`onebot.md` 第一条「场所规则和出厂数据」第 8 条，施工 O-41；O-58 上群白名单、白名单以外的群、两档限流挪进
//! 卡片，`super::cards`）：睡觉时间、群里的触发词、看不看得到 QQ 号、去句号（O-58 补二从卡片挪来）这四项是包的配置项，网页、终端的设置页照清单自己画，卡片里
//! 没设的照它们。桥照它们拼一份规则文件
//! [`FILE`]，来处是 [`Source::Settings`]，和出厂的、系统的一起交群聊内核读：写法、怎么套、写错了丢哪一项都照规则文件的规矩。
//!
//! - 一项一条规则，先后固定（[`ORDER`]）：第几条规则就是哪一项，问题、来处照它说成配置项的键（[`key_of`]）。没设、等于默认
//!   的那一项，规则照写、只有 `match`，什么都不设。
//! - 字照 JSON 的写法写进去：JSON 的转义（`\"`、`\\`、`\n`、`\u0001` 这些）都是 TOML 基本字符串认的，不另加依赖。

use serde_json::{Map, Value};

use miyu_chat::{File, Rules, Source};

use super::cards::GroupSet;
use crate::PACKAGE;

/// 拼出来的那一份的文件名：排在出厂的 `50-defaults.toml` 后面；系统里名字排在它后面的盖它，同名的整份替换它。
pub const FILE: &str = "60-settings.toml";

/// 四项在清单里的名字，照拼出来的规则的先后：第 n 条规则是第 n 项。
const ORDER: [&str; 4] = [KEYWORDS, SLEEP, SHOW_IDS, STRIP_PERIOD];

/// 睡觉时间：写法同规则的 `sleep`，群和私聊都算。
const SLEEP: &str = "sleep";

/// 群里的触发词：字的列表。
const KEYWORDS: &str = "keywords";

/// 她看得到发话人的 QQ 号：开关，默认关。
const SHOW_IDS: &str = "show_ids";

/// 白名单成员（施工 O-60）：不拼进规则，「其他私聊」卡照它排开。
const WHITELIST: &str = "whitelist";

/// 去掉末尾的句号（施工 O-58 补二，从卡片挪来：项目主人 10-19 定是通用配置）：开关，默认开（同出厂参数），关了的拼进规则。
const STRIP_PERIOD: &str = "strip_period";

/// 四项这一刻的值：握手交来的，推来变了的照换。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Configured {
    /// 睡觉时间，原文；没设的是空的。
    sleep: Option<String>,
    /// 群里的触发词。
    keywords: Vec<String>,
    /// 她看得到 QQ 号。
    show_ids: bool,
    /// 去掉末尾的句号。
    strip_period: bool,
    /// 白名单成员的号（`im.whitelist` 里这个平台的，施工 O-60）：不拼进规则，「其他私聊」卡不管他们。
    whitelist: Vec<u64>,
}

impl Default for Configured {
    fn default() -> Configured {
        Configured {
            sleep: None,
            keywords: Vec::new(),
            show_ids: false,
            strip_period: true,
            whitelist: Vec::new(),
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
    /// 键不认。交回四项变了没有。
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
                SLEEP => self.sleep = text(value),
                KEYWORDS => self.keywords = texts(value),
                SHOW_IDS => self.show_ids = value.as_bool().unwrap_or(fallback.show_ids),
                STRIP_PERIOD => {
                    self.strip_period = value.as_bool().unwrap_or(fallback.strip_period);
                }
                WHITELIST => {
                    self.whitelist = texts(value)
                        .iter()
                        .filter_map(|one| one.strip_prefix("qq:")?.parse().ok())
                        .collect();
                }
                _ => {}
            }
        }
        *self != before
    }

    /// 挪进会话默认值（施工 I-7 下二）：睡觉时间、群里的触发词、看得到 QQ 号（开着的才算设了）这三项，设了、认得出的写进
    /// `defaults` 那一格还没写的（卡上写了的照卡），交回挪了的几项在清单里的名字（配置里要去掉的）。没认出来的（[`Self::misread`]）
    /// 不挪：留在配置里照旧报，改对了下次起来再挪。
    pub fn move_into(&self, defaults: &mut GroupSet) -> Vec<&'static str> {
        let misread = self.misread();
        let readable = |name: &str| !misread.contains(&format!("{PACKAGE}.{name}"));
        let mut moved = Vec::new();
        if let Some(sleep) = self.sleep.as_ref().filter(|_| readable(SLEEP)) {
            moved.push(SLEEP);
            defaults.sleep.get_or_insert_with(|| sleep.clone());
        }
        if !self.keywords.is_empty() && readable(KEYWORDS) {
            moved.push(KEYWORDS);
            defaults
                .keywords
                .get_or_insert_with(|| self.keywords.clone());
        }
        if self.show_ids && readable(SHOW_IDS) {
            moved.push(SHOW_IDS);
            defaults.show_ids.get_or_insert(true);
        }
        moved
    }

    /// 拼成规则文件（来处 [`Source::Settings`]，名字 `60-settings.toml`），四条规则照固定的先后（[`key_of`]）。
    pub fn file(&self) -> File {
        let all_groups = "{ kind = \"group\" }".to_string();
        let mut text = String::new();
        for name in ORDER {
            let (matcher, attribute) = match name {
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
                // 开着是出厂参数本来的样子：关了才写。
                _ => (
                    None,
                    (!self.strip_period)
                        .then(|| ("outbound", "{ strip_period = false }".to_string())),
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

impl Configured {
    /// 白名单成员的号（施工 O-60：「其他私聊」卡不管他们）。
    pub fn whitelisted(&self) -> &[u64] {
        &self.whitelist
    }

    /// 没认出来的配置项（施工 O-41 补）：拼出来的这一份自己交群聊内核读，有问题的那几条规则是哪几项，配置项的键照拼的先后。
    /// 只看这一份：规则文件写错的不算。后台页的 `status`、状态文件照它说。一条规则只设一项、一项写错只报一处（列表里坏了几个
    /// 也是一处），不用去重。
    pub fn misread(&self) -> Vec<String> {
        Rules::parse(&[self.file()])
            .problems
            .iter()
            .filter_map(|problem| problem.rule.and_then(key_of))
            .collect()
    }
}

/// 拼出来的第 `rule` 条规则（从 1 数）是哪一项的：配置项的键（`im.sleep`）；不在四条里的是空的。
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

#[cfg(test)]
mod tests;
