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
//! - `admin_kick`：踢人工具的开关，默认开（施工 Q-2）。
//! - `admin_records`：记不记管理记录，默认开；`admin_keep`：每个群留多少条，出厂 500（施工 Q-2，`records`）。
//! - `admin_log`：查管理记录的工具的开关；`admin_summary`：成员汇总（排行榜）的开关，都默认开（施工 Q-4）。
//! - `admin_sync_unmute`：别的管理员解禁的也记、也告诉 AI，默认开（施工 Q-3，`sync`）。
//! - `admin_mute_min`、`admin_mute_max`：最短、最长禁言几分钟（出厂 1、43200，就是 QQ 能禁的最短、最长）。她给的不在这中间的答
//!   `bad-seconds`，带这两个时长；解禁（0 秒）不管。读不出的照出厂的。

use serde_json::{Map, Value};

use miyu_im::chat::{CallError, Link, Rank};
use miyu_im::plugins::declared::{Links, Page, Ui};
use miyu_im::plugins::{Builtin, Called, Context, Declared};

pub(crate) mod actions;
pub(crate) mod approval;
pub(crate) mod groups;
pub(crate) mod page;
pub(crate) mod records;
pub(crate) mod spam;
pub(crate) mod summary;
pub(crate) mod sync;

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

/// 踢人工具的开关（施工 Q-2）。
pub(crate) const KICK: &str = "admin_kick";

/// 记录开关（施工 Q-2）。
pub(crate) const RECORDS: &str = "admin_records";

/// 每个群留多少条记录（施工 Q-2）。
pub(crate) const KEEP: &str = "admin_keep";

/// 查管理记录的工具的开关（施工 Q-4）。
pub(crate) const LOG: &str = "admin_log";

/// 成员汇总（排行榜）的开关（施工 Q-4）。
pub(crate) const SUMMARY: &str = "admin_summary";

/// 同步群里别的管理员的解禁（施工 Q-3）。
pub(crate) const SYNC_UNMUTE: &str = "admin_sync_unmute";

/// 每个群留多少条读不出时照的（同清单的出厂值，同旧版）。
const KEEP_DEFAULT: usize = 500;

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
            // 会话浮窗的「群管理」页、面板（施工 Q-6，设计 02 第四节第 4、5 条）。
            ui: Ui {
                home: false,
                panel: true,
                pages: vec![Page {
                    id: "admin".to_string(),
                    title: words("群管理面板", "Group admin panel", "グループ管理パネル"),
                    method: "admin.page".to_string(),
                }],
            },
        }
    }

    fn settings(&self) -> &'static str {
        PREFIX
    }

    fn call<'a>(&'a self, context: &'a Context, method: &'a str, params: &'a Value) -> Called<'a> {
        Box::pin(actions::call(context, method, params))
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

/// 踢人工具登记不登记（施工 Q-2）：总开关、踢人工具的开关都开着。
pub(crate) fn offers_kick(config: &Map<String, Value>) -> bool {
    switched(config, ON) && switched(config, KICK)
}

/// 查管理记录的工具登记不登记（施工 Q-4）：总开关、这件工具的开关都开着。
pub(crate) fn offers_log(config: &Map<String, Value>) -> bool {
    switched(config, ON) && switched(config, LOG)
}

/// 成员汇总（排行榜）开着没有（施工 Q-4）。
pub(crate) fn summarizing(config: &Map<String, Value>) -> bool {
    switched(config, SUMMARY)
}

/// 记不记管理记录（施工 Q-2）。
pub(crate) fn recording(config: &Map<String, Value>) -> bool {
    switched(config, RECORDS)
}

/// 每个群留多少条记录：读不出的、0 照出厂的。
pub(crate) fn keep(config: &Map<String, Value>) -> usize {
    config
        .get(KEEP)
        .and_then(Value::as_u64)
        .and_then(|keep| usize::try_from(keep).ok())
        .filter(|keep| *keep > 0)
        .unwrap_or(KEEP_DEFAULT)
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

/// 她（机器人号 `bot`）在群 `group` 里管得了人没有：问 NapCat 她的身份，群主、管理员才算（施工 Q-1）。不用 NapCat 的缓存（施工
/// I-17：刚给、刚撤的管理员照旧报回来）。问不到的交回为什么；回成了却没有她的不算。
pub(crate) async fn admin_here(link: &dyn Link, group: i64, bot: i64) -> Result<bool, CallError> {
    let me = crate::onebot::fresh_member(link, group, bot).await?;
    Ok(me.is_some_and(|me| ranked(me.rank)))
}

/// 群里的身份 `rank` 管得了人没有：群主、管理员。
pub(crate) fn ranked(rank: Option<Rank>) -> bool {
    matches!(rank, Some(Rank::Owner | Rank::Admin))
}

#[cfg(test)]
mod tests;
