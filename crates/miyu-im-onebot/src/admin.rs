//! 群管理（施工 Q-1 起，设计 02；2026-10-20 项目主人定：群管理内置进 OneBot 驱动，同识人术）：OneBot 驱动带的 im 插件
//! `miyu-im-qq-admin`。禁言、踢人、查管理记录这几件工具在驱动的工具套里（`crate::tools`），这里是插件的声明、设置和生效的前提。
//!
//! 生效的前提（设计 02 第四节第 2 条）：插件开着（总开关、这个群的开关，im 交的 `Call::plugins`），并且她在这个群里是管理员或群主
//! （[`ranked`]）。不满足的群里工具答 `admin-off`。
//!
//! 设置是驱动的配置项（`im.qq.admin_*`，藏起来，在插件页「设置」里改）：
//!
//! - `admin_on`：总开关，默认开。关了的群管理的工具都不登记（第 3 条：改开关时 im 重新登记）。
//! - `admin_mute`：禁言工具的开关，默认开。关了的不登记禁言。
//! - `admin_mute_min`、`admin_mute_max`：最短、最长禁言几分钟（出厂 1、43200，就是 QQ 能禁的最短、最长）。她给的不在这中间的答
//!   `bad-seconds`，带这两个时长；解禁（0 秒）不管。读不出的照出厂的。

use serde_json::{Map, Value};

use miyu_im::chat::Rank;
use miyu_im::plugins::declared::{Links, Ui};
use miyu_im::plugins::{Builtin, Declared};

/// 编号。
pub(crate) const ID: &str = "miyu-im-qq-admin";

/// 设置项名字的开头。
const PREFIX: &str = "admin_";

/// 总开关。
pub(crate) const ON: &str = "admin_on";

/// 禁言工具的开关。
pub(crate) const MUTE: &str = "admin_mute";

/// 最短禁言。
pub(crate) const MUTE_MIN: &str = "admin_mute_min";

/// 最长禁言。
pub(crate) const MUTE_MAX: &str = "admin_mute_max";

/// 最短禁言读不出时照的（同清单的出厂值）：一分钟，QQ 能禁的最短。
const MIN_SECONDS: u64 = 60;

/// 最长禁言读不出时照的（同清单的出厂值）：30 天，QQ 能禁的最长。
const MAX_SECONDS: u64 = 2_592_000;

/// 群管理。
pub struct Admin;

impl Builtin for Admin {
    fn id(&self) -> &'static str {
        ID
    }

    fn declared(&self) -> Declared {
        let words = |zh: &str, en: &str, ja: &str| {
            [("zh", zh), ("en", en), ("ja", ja)]
                .into_iter()
                .map(|(lang, text)| (lang.to_string(), text.to_string()))
                .collect()
        };
        Declared {
            name: words("群管理", "Group admin", "グループ管理"),
            summary: words(
                "AI 是管理员的群里，让 AI 禁言、踢人、查管理记录。",
                "In groups where the AI is an admin, lets the AI mute, kick and look up moderation records.",
                "AI が管理者のグループで、AI がミュート、退出、管理記録の確認をできるようにします。",
            ),
            links: Links {
                icon: Some("shield".to_string()),
                ..Links::default()
            },
            platforms: vec!["qq".to_string()],
            needs: Vec::new(),
            default: true,
            ui: Ui::default(),
        }
    }

    fn settings(&self) -> &'static str {
        PREFIX
    }
}

/// 驱动的配置 `config` 里的开关 `name`：没写、不是布尔的当开着。
pub(crate) fn switched(config: &Map<String, Value>, name: &str) -> bool {
    config.get(name).and_then(Value::as_bool).unwrap_or(true)
}

/// 禁言工具登记不登记（施工 Q-1）：总开关、禁言工具的开关都开着。
pub(crate) fn offers_mute(config: &Map<String, Value>) -> bool {
    switched(config, ON) && switched(config, MUTE)
}

/// 最短、最长禁言几秒（纯逻辑）：设置是分钟，读不出的、不是正整数的照出厂的，超过 30 天的照 30 天；最短比最长还长的，两个都照出厂的。
pub(crate) fn mute_range(config: &Map<String, Value>) -> (u64, u64) {
    let read = |name: &str| {
        config
            .get(name)
            .and_then(Value::as_u64)
            .filter(|minutes| *minutes > 0)
            .and_then(|minutes| minutes.checked_mul(60))
    };
    let min = read(MUTE_MIN).unwrap_or(MIN_SECONDS);
    let max = read(MUTE_MAX).unwrap_or(MAX_SECONDS).min(MAX_SECONDS);
    if min > max {
        return (MIN_SECONDS, MAX_SECONDS);
    }
    (min, max)
}

/// 群里的身份 `rank` 管得了人没有：群主、管理员。
pub(crate) fn ranked(rank: Option<Rank>) -> bool {
    matches!(rank, Some(Rank::Owner | Rank::Admin))
}

#[cfg(test)]
mod tests;
