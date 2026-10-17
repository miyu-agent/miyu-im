//! 挂接点（施工 O-43 上，`onebot.md` 第一条「挂接点」，`extending.md` 第二条）：别的包导出约定名字的方法，桥在那一处经核心的
//! `package.call` 调它。别的包不用另外登记：桥订阅核心的软件包列表（`subscribe {"stream": "packages"}`、推来的
//! `package.changed`），照每一项的 `export.methods` 记下谁导出了哪一处，在跑的才算（`status` 是 `running`；缺了依赖的核心不拉起，
//! 是 `depends_missing`）。先后照包的编号。
//!
//! 这一步只有一处：[`EVENT`]，NapCat 推来的通知、请求原样交出去，不等回应、不看回答（[`Hooks::event`]）。

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};

use super::caller::Caller;
use super::reason;
use crate::TARGET;

/// QQ 事件：NapCat 推来的 `notice`、`request` 原样交 `{"bot", "event"}`，不等回应。
pub(crate) const EVENT: &str = "qq.event";

/// 桥认的挂接点：别的包导出了这里面的名字才算挂上。
const POINTS: [&str; 1] = [EVENT];

/// 谁挂在哪：包的编号到它导出的挂接点。
pub(crate) struct Hooks {
    /// 经它发 `package.call`：并着发，不等跟核心的那一头手上的事。
    caller: Caller,
    /// 在跑、导出了挂接点的包，照编号排。
    hooked: Mutex<BTreeMap<String, Vec<&'static str>>>,
}

impl Hooks {
    /// 还没看过软件包列表的一份：哪一处都没挂。
    pub(crate) fn new(caller: Caller) -> Hooks {
        Hooks {
            caller,
            hooked: Mutex::new(BTreeMap::new()),
        }
    }

    /// 照软件包列表 `packages`（订阅的回应、`package.list` 的 `packages`）整份换上。
    pub(crate) fn learn(&self, packages: &Value) {
        let hooked = table(packages);
        tracing::info!(target: TARGET, count = hooked.len(), "hooks learned");
        *self.lock() = hooked;
    }

    /// 推来的一项（`package.changed` 的 `package`、`entry`）：照它换那一个包（[`change`]）。
    pub(crate) fn changed(&self, package: &str, entry: &Value) {
        change(&mut self.lock(), package, entry);
    }

    /// 挂在 `point` 的包，照编号排。
    pub(crate) fn at(&self, point: &str) -> Vec<String> {
        hooked_at(&self.lock(), point)
    }

    /// NapCat 推来的一件通知、请求 `event`（原样，机器人的号是 `bot`）交给挂在 [`EVENT`] 的每一个包：各起一个小任务发，不等
    /// 回应；核心拒了的、别的包答错的记一行 `WARN hook failed`。没有挂的什么都不做。
    pub(crate) fn event(&self, bot: i64, event: &Value) {
        for package in self.at(EVENT) {
            let caller = self.caller.clone();
            let params = json!({"package": package, "method": EVENT, "params": {"bot": bot, "event": event}});
            tokio::spawn(async move {
                if let Ok(reply) = caller.call("package.call", params).await
                    && let Some(why) = reason(&reply)
                {
                    tracing::warn!(target: TARGET, package, point = EVENT, why, "hook failed");
                }
                // 核心断开（`Gone`）：跟核心的那一头会读到头、桥停下，这里没有别的要做。
            });
        }
    }

    /// 锁上表。锁坏了照样用：表里只有编号和名字。
    fn lock(&self) -> MutexGuard<'_, BTreeMap<String, Vec<&'static str>>> {
        self.hooked.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// 软件包列表 `packages` 里挂了的：包的编号到它挂的几处；没挂的、没有编号的不在里面。
fn table(packages: &Value) -> BTreeMap<String, Vec<&'static str>> {
    packages
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| Some((entry["package"].as_str()?.to_string(), points(entry))))
        .filter(|(_, points)| !points.is_empty())
        .collect()
}

/// 表 `hooked` 照推来的一项换包 `package`：挂了的换上它挂的几处；`entry` 是 `null` 的（卸掉了）、不在跑了、不导出了的拿掉。
fn change(hooked: &mut BTreeMap<String, Vec<&'static str>>, package: &str, entry: &Value) {
    let points = points(entry);
    if points.is_empty() {
        hooked.remove(package);
    } else {
        hooked.insert(package.to_string(), points);
    }
}

/// 表 `hooked` 里挂在 `point` 的包，照编号排。
fn hooked_at(hooked: &BTreeMap<String, Vec<&'static str>>, point: &str) -> Vec<String> {
    hooked
        .iter()
        .filter(|(_, points)| points.contains(&point))
        .map(|(package, _)| package.clone())
        .collect()
}

/// 软件包列表的一项挂在哪几处：在跑的，`export.methods` 里桥认的名字，照 [`POINTS`] 的先后；不在跑的、没导出的、`null` 是空的。
fn points(entry: &Value) -> Vec<&'static str> {
    if entry["status"] != "running" {
        return Vec::new();
    }
    let exported: Vec<&str> = entry["export"]["methods"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    POINTS
        .into_iter()
        .filter(|point| exported.contains(point))
        .collect()
}

#[cfg(test)]
mod tests;
