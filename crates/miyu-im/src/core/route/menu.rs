//! 命令菜单（施工 I-183 下上，项目主人 11-09：Telegram 私聊里打 `/` 列出 Miyu 的命令）：号连上时问核心有哪些斜杠命令（
//! `command.catalog`）、这个平台上哪几个号是主人（`config.get` 里 `external.bindings` 值不空的 `<平台>:<号>`），交给那个号的连接
//! （`Link::menu`），只给主人们看。平台没有命令菜单的（QQ）驱动答做不到，不记；问不到、交不成的记一行，不影响别的。在旁边跑，
//! 不挡着收发。

use serde_json::{Value, json};

use super::Route;
use crate::TARGET;
use crate::chat::{CallError, Cap, Id, MenuItem, Platform};
use crate::core::reason;

/// 主人对应表在配置里的前缀。
const BINDINGS: &str = "external.bindings.";

impl Route {
    /// 平台 `platform` 上号是 `bot` 的那个连上了：在旁边问好命令和主人，交给它的连接。
    pub(super) fn tell_menu(&mut self, platform: Platform, bot: Id) {
        // 没有命令菜单的平台（QQ）不问核心。
        if !self.platforms.can(platform, Cap::Menu) {
            return;
        }
        let Some(link) = self.platforms.link(platform, &bot) else {
            return;
        };
        let caller = self.core.caller();
        self.chores.spawn(async move {
            let catalog = match caller.call("command.catalog", json!({})).await {
                Ok(answered) if reason(&answered).is_none() => answered,
                Ok(answered) => {
                    tracing::warn!(target: TARGET, why = reason(&answered), "command menu not built");
                    return;
                }
                Err(_) => return,
            };
            let config = match caller.call("config.get", json!({})).await {
                Ok(answered) if reason(&answered).is_none() => answered,
                _ => return,
            };
            let owners = owners(&config["result"], platform);
            let items = items(&catalog["result"]);
            match link.menu(&owners, &items).await {
                Ok(()) => tracing::info!(target: TARGET, bot = %bot, owners = owners.len(), commands = items.len(), "command menu set"),
                Err(CallError::Unsupported) => {}
                Err(error) => tracing::warn!(target: TARGET, bot = %bot, error = ?error, "command menu not set"),
            }
        });
    }
}

/// `config.get` 的回应里这个平台上的主人：`external.bindings."<平台>:<号>"` 值不空的。
pub(super) fn owners(config: &Value, platform: Platform) -> Vec<Id> {
    let prefix = format!("{}:", platform.name());
    config["items"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, item)| {
            item["value"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        })
        .filter_map(|(key, _)| {
            let identity = key.strip_prefix(BINDINGS)?;
            // 核心照 TOML 的写法给键加了引号（`"tg:7581443271"`）。
            let identity = identity
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
                .unwrap_or(identity);
            Id::new(identity.strip_prefix(&prefix)?)
        })
        .collect()
}

/// `command.catalog` 的回应读成菜单的几项：名字、说明。
pub(super) fn items(catalog: &Value) -> Vec<MenuItem> {
    catalog["commands"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|one| {
            Some(MenuItem {
                name: one["name"].as_str()?.to_string(),
                summary: one["summary"].as_str().unwrap_or_default().to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
