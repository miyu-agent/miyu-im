//! 这个包的配置项（清单 `[settings]`）：握手时核心交最终值，变了推 `extension.config`，只放变了的键、变成没设的是 `null`。键是
//! 包的编号加名字。

use serde_json::{Map, Value};

/// 包的编号：配置项的键照它开头。
pub const PACKAGE: &str = "miyu-im-sponsor";

/// 出厂的默认币种。
const CURRENCY: &str = "CNY";

/// 现在的值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// 默认币种（设计 03 第一节第 4 条）：别的币种照记账那一刻的汇率换成它。
    pub currency: String,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            currency: CURRENCY.to_string(),
        }
    }
}

impl Settings {
    /// 照 `keys` 换：认得的换上，`null` 的、写错的回出厂值，别的不理。
    pub fn apply(&mut self, keys: &Map<String, Value>) {
        for (key, value) in keys {
            let name = key
                .strip_prefix(PACKAGE)
                .and_then(|rest| rest.strip_prefix('.'));
            if name == Some("currency") {
                self.currency = value
                    .as_str()
                    .and_then(currency)
                    .unwrap_or_else(|| CURRENCY.to_string());
            }
        }
    }
}

/// 一个币种的写法（纯逻辑）：三个英文字母，写成大写；别的是空的。
pub fn currency(text: &str) -> Option<String> {
    let text = text.trim();
    (text.len() == 3 && text.bytes().all(|byte| byte.is_ascii_alphabetic()))
        .then(|| text.to_ascii_uppercase())
}
