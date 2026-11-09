//! @ 指定的人的参数和认人（施工 O-55、I-17；施工 I-183 下下从 OneBot 驱动挪来，`qq_mention`、`tg_mention` 同用）：纯逻辑。成员从哪认
//! 各平台自己办（QQ 取群成员表，Telegram 照 im 的聊天记录认说过话的人），认好的交 [`named`] 拼名字。

use std::collections::HashMap;

use serde_json::Value;

use crate::chat::Id;
use crate::toolset::{NAME, clean};

use super::args::numbers;

/// 一次最多 @ 几个人（施工 I-17，同旧版 `MAX_TOOL_MENTIONS`）。
pub const MOST: usize = 32;

/// 读参数 `args` 里的号（纯逻辑）：`users` 是正整数（写成字的也认）的列表，去重、照先后，1 到 [`MOST`] 个；别的是空的。
#[must_use]
pub fn users(args: &Value) -> Option<Vec<Id>> {
    numbers(&args["users"], MOST)
}

/// 照缓存着的名字 `cached`、成员表 `listed` 认 `users`（纯逻辑）：交回（成员的名字，照先后；不是成员的号）。名字洗过，洗完空了的
/// 写号。
#[must_use]
pub fn named(
    users: &[Id],
    cached: &HashMap<Id, String>,
    listed: &HashMap<Id, Option<String>>,
) -> (Vec<String>, Vec<String>) {
    let mut names = Vec::new();
    let mut strangers = Vec::new();
    for user in users {
        let name = match (cached.get(user), listed.get(user)) {
            (Some(name), _) => Some(Some(name.clone())),
            (None, Some(name)) => Some(name.as_deref().and_then(|name| clean(name, NAME))),
            (None, None) => None,
        };
        match name {
            Some(name) => names.push(name.unwrap_or_else(|| user.to_string())),
            None => strangers.push(user.to_string()),
        }
    }
    (names, strangers)
}

#[cfg(test)]
mod tests;
