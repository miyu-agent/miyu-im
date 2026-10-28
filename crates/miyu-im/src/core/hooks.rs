//! 挂接点（施工 O-43 上，`onebot.md` 第一条「挂接点」，`extending.md` 第二条）：别的包导出约定名字的方法，桥在那一处经核心的
//! `package.call` 调它。别的包不用另外登记：桥订阅核心的软件包列表（`subscribe {"stream": "packages"}`、推来的
//! `package.changed`），照每一项的 `export.methods` 记下谁导出了哪一处，在跑的才算（`status` 是 `running`；缺了依赖的核心不拉起，
//! 是 `depends_missing`）。先后照包的编号。
//!
//! - [`EVENT`]（施工 O-43 上）：NapCat 推来的通知、请求原样交出去，不等回应、不看回答（[`Hooks::event`]）。
//! - [`INBOUND`]（施工 O-43 中）：群里一条人说的话记下以后、判以前，并着问挂在这里的每一个包，每个最多等 `hook_millis`；回
//!   `record`（只记下）、`reply`（当成冲她来的直接回）、`pass`（照常），几个包的照 [`Steer`] 的轻重取最重的（[`Hooks::inbound`]）。
//! - [`OUTBOUND`]（施工 O-43 下）：她的一段话过出站链以前，照包的编号一个接一个问，前一个换了的交给后一个；回 `{"text"}` 换、
//!   `{"drop": true}` 不发、`{}` 照原样（[`Hooks::outbound`]）。
//! - [`SCORE`]（施工 O-43 末）：要问判官打分的那一条，判官答了以后并着问，回 `{"bonus": <-1 到 1>}`，几个包加起来再夹到 -1 到 1，
//!   进算分的 `bonus`（[`Hooks::score`]）。

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use futures_util::future::join_all;
use serde_json::{Map, Value, json};
use tokio::sync::watch;

use super::caller::Caller;
use super::reason;
use crate::TARGET;
use miyu_chat::VenueKind;

use crate::chat::{Cap, Platform, To};
use crate::plugins::{Builtin, Plugins, Toggles};
use answers::{Edit, heaviest, summed};
use follow::Followed;

/// QQ 事件：NapCat 推来的 `notice`、`request` 原样交 `{"bot", "event"}`，不等回应。
pub(crate) const EVENT: &str = "chat.event";

/// 进站：群里一条人说的话判以前问，回 `{"route": "pass" | "record" | "reply"}`。
pub(crate) const INBOUND: &str = "chat.inbound";

/// 出站：她的一段话发出去以前问，回 `{"text": <换成的>}`、`{"drop": true}` 或 `{}`。
pub(crate) const OUTBOUND: &str = "chat.outbound";

/// 判断打分：判官答了、算分以前问，回 `{"bonus": <-1 到 1 的数>}`。
pub(crate) const SCORE: &str = "chat.score";

/// 交给挂接点的参数 `params`（一个对象）带上平台（施工 I-1）：插件照它知道是哪个平台的。不是对象的照原样。
fn with_platform(params: &Value, platform: Platform) -> Value {
    let mut params = params.clone();
    if let Value::Object(fields) = &mut params {
        fields.insert("platform".into(), Value::from(platform.name()));
    }
    params
}

/// 桥认的挂接点：别的包导出了这里面的名字才算挂上。
pub(crate) const POINTS: [&str; 4] = [EVENT, INBOUND, OUTBOUND, SCORE];

/// 问过判断打分挂接点的结果（施工 O-43 末）。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Nudge {
    /// 加起来、夹过的加分。
    pub(crate) bonus: f64,
    /// 给了不是 0 的加分的包，照编号排。
    pub(crate) by: Vec<String>,
}

/// 问过出站挂接点的结果（施工 O-43 下）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Voiced {
    /// 照这一段发：没换的是原样，换了的是最后换成的。
    Say(String),
    /// 这个包说不发。
    Drop(String),
}

/// 进站挂接点的意见，照轻重排：几个包的取最重的，只记下盖直接回盖照常（施工 O-43 中）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Steer {
    /// 照常：没意见、等不到、答错的都是它。
    Pass,
    /// 当成冲她来的直接回（不越过进站链：限流、睡觉、不让叫她的群照旧）。
    Reply,
    /// 只记下，不叫她（终端管理员的照旧）。
    Record,
}

impl Steer {
    /// 回答里 `route` 的写法；认不出的是空的。
    fn read(route: &Value) -> Option<Steer> {
        match route.as_str()? {
            "pass" => Some(Steer::Pass),
            "reply" => Some(Steer::Reply),
            "record" => Some(Steer::Record),
            _ => None,
        }
    }

    /// 写进判断的名字。
    pub(crate) fn name(self) -> &'static str {
        match self {
            Steer::Pass => "pass",
            Steer::Reply => "reply",
            Steer::Record => "record",
        }
    }
}

/// 问过进站挂接点的结果：最重的意见、给出它的包（照编号排头一个；照常的没有）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Steered {
    /// 意见。
    pub(crate) steer: Steer,
    /// 给出这个意见的包。
    pub(crate) by: Option<String>,
}

impl Steered {
    /// 没意见：没有挂的、都照常的。
    pub(crate) fn pass() -> Steered {
        Steered {
            steer: Steer::Pass,
            by: None,
        }
    }
}

/// 一个驱动交给挂接点的（施工 I-70 中上，[`Hooks::new`]）：插件在它的平台上的场所里能不能用照它的能力表算（施工 I-8 上），它带的
/// 插件（施工 I-10 上，[`builtins`]）的开关、设置照它这一刻的配置。
pub(crate) struct Mounted {
    /// 驱动的平台。
    pub(crate) platform: Platform,
    /// 驱动的能力表。
    pub(crate) caps: &'static [Cap],
    /// 驱动带的插件。
    pub(crate) builtins: Vec<Arc<dyn Builtin>>,
    /// 驱动这一刻的配置（`Current::driver`）。
    pub(crate) config: watch::Receiver<Map<String, Value>>,
}

/// 谁挂在哪：包的编号到它导出的挂接点。
pub(crate) struct Hooks {
    /// 经它发 `package.call`：并着发，不等跟核心的那一头手上的事。
    caller: Caller,
    /// 问进站挂接点的包每一次最多等多久（`bridge.json` 的 `hook_millis`，施工 O-43 中）。
    wait: Duration,
    /// 挂着的驱动，照先后（施工 I-70 中上）：插件在一个场所能不能用照那个场所的平台的驱动算，交给挂接点的参数带的平台照每一次的
    /// （施工 I-70 上）；只说一个平台的（插件页的 [`Hooks::listed`]）照第一个。
    drivers: Vec<Mounted>,
    /// 认得的插件（施工 I-8 上）：不是插件的包不问。
    plugins: Mutex<Plugins>,
    /// 场所开关（施工 I-8 上）：只问在那个场所开着的。
    toggles: Mutex<Toggles>,
    /// 在跑、导出了挂接点的包，照编号排；跟着软件包列表走的先后（[`follow`]）。
    hooked: Mutex<Followed>,
}

impl Hooks {
    /// 还没看过软件包列表的一份：哪一处都没挂，只认得挂着的驱动 `drivers` 带的插件；问进站挂接点每个最多等 `wait`。
    pub(crate) fn new(caller: Caller, wait: Duration, drivers: Vec<Mounted>) -> Hooks {
        let plugins = Plugins::default().with_builtins(&builtins_of(&drivers));
        Hooks {
            caller,
            wait,
            drivers,
            plugins: Mutex::new(plugins),
            toggles: Mutex::new(Toggles::default()),
            hooked: Mutex::new(Followed::default()),
        }
    }

    /// 一次订阅软件包列表要发出去了：领它的号，回应到了交给 [`Self::learn`]（[`follow`]）。
    pub(crate) fn subscribing(&self) -> u64 {
        self.lock().subscribing()
    }

    /// 第 `ticket` 次订阅的回应 `packages`：整份换上，这次订阅以后推来的照办；比换上的旧的不要（[`follow`]）。
    pub(crate) fn learn(&self, ticket: u64, packages: &Value) {
        let mut followed = self.lock();
        if followed.learn(ticket, packages) {
            tracing::info!(target: TARGET, count = followed.hooked.len(), "hooks learned");
        }
    }

    /// 推来的一项（`package.changed` 的 `package`、`entry`）：照它换那一个包（[`change`]）。挂的变了记一行 `INFO hooks changed`
    /// （包、现在挂了几个）。
    pub(crate) fn changed(&self, package: &str, entry: &Value) {
        let mut followed = self.lock();
        let before = followed.hooked.get(package).cloned();
        followed.changed(package, entry);
        if followed.hooked.get(package) != before.as_ref() {
            tracing::info!(target: TARGET, package, count = followed.hooked.len(), "hooks changed");
        }
    }

    /// 认得的插件换成 `plugins`（施工 I-8 上：起来时、核心推 `content.changed` 时读的）。
    pub(crate) fn set_plugins(&self, plugins: Plugins) {
        let plugins = plugins.with_builtins(&builtins_of(&self.drivers));
        *self.plugins.lock().unwrap_or_else(PoisonError::into_inner) = plugins;
    }

    /// 认得的 im 插件（会话浮窗「管理插件」、插件页，[`listed`]）：能不能用照第一个驱动的平台算（页面分平台是 I-70 下的事）。一个
    /// 驱动都没挂的（照说不会）是空的。
    pub(crate) fn listed(&self) -> Vec<Value> {
        let Some(first) = self.drivers.first() else {
            return Vec::new();
        };
        let plugins = self.plugins.lock().unwrap_or_else(PoisonError::into_inner);
        listed::listed(&plugins, first.platform, first.caps)
    }

    /// 平台 `platform` 的驱动；没挂的是空的。
    fn mounted(&self, platform: Platform) -> Option<&Mounted> {
        self.drivers.iter().find(|one| one.platform == platform)
    }

    /// 场所开关换成 `toggles`（施工 I-8 上：卡片读进来、存下时拼的）。
    pub(crate) fn set_toggles(&self, toggles: Toggles) {
        *self.toggles.lock().unwrap_or_else(PoisonError::into_inner) = toggles;
    }

    /// 平台 `platform` 上发到 `to` 的那个场所里挂在 `point` 的包，照编号排：在跑、导出了它（软件包列表），是 im 的插件、在这个
    /// 平台上能用、在这个场所开着（施工 I-8 上）。场所认不出的（`to` 是 `None`）照插件声明的默认。
    pub(crate) fn at(&self, point: &str, platform: Platform, to: Option<To>) -> Vec<String> {
        let hooked = hooked_at(&self.lock().hooked, point);
        hooked
            .into_iter()
            .filter(|package| self.allowed(package, platform, to))
            .collect()
    }

    /// 包 `package` 在平台 `platform` 上发到 `to` 的那个场所里算不算开着的 im 插件（施工 I-8 上）：是插件、在这个平台上能用（照这个
    /// 平台的驱动的能力表，施工 I-70 中上；没挂的平台都不算）、这个场所开着（场所认不出的照声明的默认）。挂接点、`chat.note` 都照它。
    pub(crate) fn allowed(&self, package: &str, platform: Platform, to: Option<To>) -> bool {
        let Some(mounted) = self.mounted(platform) else {
            return false;
        };
        let plugins = self.plugins.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(declared) = plugins.declared(package) else {
            return false;
        };
        if plugins.usable(package, platform, mounted.caps).is_err() {
            return false;
        }
        let venue = to.map(|to| match to {
            To::Group(group) => (VenueKind::Group, group),
            To::Private(user) => (VenueKind::Private, user),
        });
        // 只在群、只在私聊用的插件（施工 I-51）：别的场所当关着。
        let kind = match to {
            Some(To::Group(_)) => Some("group"),
            Some(To::Private(_)) => Some("private"),
            None => None,
        };
        if kind.is_some_and(|kind| !declared.fits(kind)) {
            return false;
        }
        let toggles = self.toggles.lock().unwrap_or_else(PoisonError::into_inner);
        toggles.enabled(venue, package, declared.default)
    }

    /// 平台 `platform` 推来的一件通知、请求 `event`（原样，机器人的号是 `bot`，驱动认出的场所是 `to`）交给那个场所里挂在 [`EVENT`]
    /// 的每一个包：各起一个小任务发，不等回应；核心拒了的、别的包答错的记一行 `WARN hook failed`。没有挂的什么都不做。
    pub(crate) fn event(&self, platform: Platform, bot: i64, to: Option<To>, event: &Value) {
        for package in self.at(EVENT, platform, to) {
            let caller = self.caller.clone();
            let params = json!({"package": package, "method": EVENT, "params": {"platform": platform.name(), "bot": bot, "event": event}});
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

    /// 问挂在 [`INBOUND`] 的包 `packages`（照编号排）平台 `platform` 上的这一条 `params`：并着问，每个最多等 `hook_millis`；等不到的、核心拒了的、答错的
    /// 照照常算，记一行 `WARN hook failed package=… point=chat.inbound why=…`（`timeout`、核心的原因、`bad_answer`）。交回最重的意见
    /// 和给出它的头一个包。
    pub(crate) async fn inbound(
        &self,
        platform: Platform,
        packages: Vec<String>,
        params: Value,
    ) -> Steered {
        let wait = self.wait;
        let asks = packages.into_iter().map(|package| {
            let caller = self.caller.clone();
            let call = json!({"package": package, "method": INBOUND, "params": with_platform(&params, platform)});
            async move {
                let steer = match tokio::time::timeout(wait, caller.call("package.call", call)).await {
                    Err(_) => Err("timeout".to_string()),
                    // 核心断开：跟核心的那一头会读到头、桥停下；这一条照常。
                    Ok(Err(_)) => Ok(Steer::Pass),
                    Ok(Ok(reply)) => match reason(&reply) {
                        Some(why) => Err(why.to_string()),
                        None => Steer::read(&reply["result"]["route"]).ok_or_else(|| "bad_answer".to_string()),
                    },
                };
                let steer = steer.unwrap_or_else(|why| {
                    tracing::warn!(target: TARGET, package, point = INBOUND, why = %why, "hook failed");
                    Steer::Pass
                });
                (package, steer)
            }
        });
        let answers = join_all(asks).await;
        heaviest(answers)
    }

    /// 问挂在 [`OUTBOUND`] 的包 `packages`（照编号排）她在平台 `platform` 的场所 `venue`（机器人号 `bot`）的一段话 `text`：一个接一个问，前一个
    /// 换了的交给后一个，每个最多等 `hook_millis`；有一个说不发的就停、交回它。等不到的、核心拒了的、答错的（`text` 不是不空的
    /// 字、`drop` 不是 `true`）照原样，记一行 `WARN hook failed package=… point=chat.outbound why=…`。
    pub(crate) async fn outbound(
        &self,
        packages: Vec<String>,
        (platform, venue, bot): (Platform, &str, i64),
        text: String,
    ) -> Voiced {
        let mut text = text;
        for package in packages {
            let call = json!({"package": package, "method": OUTBOUND, "params": {"platform": platform.name(), "venue": venue, "bot": bot, "text": text}});
            let answered =
                match tokio::time::timeout(self.wait, self.caller.call("package.call", call)).await
                {
                    Err(_) => Err("timeout".to_string()),
                    // 核心断开：跟核心的那一头会读到头、桥停下；照原样。
                    Ok(Err(_)) => Ok(Value::Null),
                    Ok(Ok(reply)) => match reason(&reply) {
                        Some(why) => Err(why.to_string()),
                        None => Ok(reply["result"].clone()),
                    },
                };
            match answered.and_then(|result| Edit::read(&result)) {
                Ok(Edit::Drop) => return Voiced::Drop(package),
                Ok(Edit::Change(changed)) => text = changed,
                Ok(Edit::Keep) => {}
                Err(why) => {
                    tracing::warn!(target: TARGET, package, point = OUTBOUND, why = %why, "hook failed");
                }
            }
        }
        Voiced::Say(text)
    }

    /// 问挂在 [`SCORE`] 的包 `packages`（照编号排）平台 `platform` 上的这一条 `params`：并着问，每个最多等 `hook_millis`；回 `{"bonus": <数>}`，每个
    /// 夹到 -1 到 1，加起来再夹一次。等不到的、核心拒了的、答错的（`bonus` 不是有限的数）照 0 算，记一行 `WARN hook failed
    /// package=… point=chat.score why=…`。
    pub(crate) async fn score(
        &self,
        platform: Platform,
        packages: Vec<String>,
        params: Value,
    ) -> Nudge {
        let wait = self.wait;
        let asks = packages.into_iter().map(|package| {
            let caller = self.caller.clone();
            let call = json!({"package": package, "method": SCORE, "params": with_platform(&params, platform)});
            async move {
                let bonus = match tokio::time::timeout(wait, caller.call("package.call", call)).await {
                    Err(_) => Err("timeout".to_string()),
                    // 核心断开：跟核心的那一头会读到头、桥停下；照 0 算。
                    Ok(Err(_)) => Ok(0.0),
                    Ok(Ok(reply)) => match reason(&reply) {
                        Some(why) => Err(why.to_string()),
                        None => reply["result"]["bonus"]
                            .as_f64()
                            .filter(|bonus| bonus.is_finite())
                            .ok_or_else(|| "bad_answer".to_string()),
                    },
                };
                let bonus = bonus.unwrap_or_else(|why| {
                    tracing::warn!(target: TARGET, package, point = SCORE, why = %why, "hook failed");
                    0.0
                });
                (package, bonus)
            }
        });
        summed(join_all(asks).await)
    }

    /// 锁上表。锁坏了照样用：表里只有编号和名字。
    fn lock(&self) -> MutexGuard<'_, Followed> {
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

/// 挂着的驱动 `drivers` 带的插件，记着是哪个驱动的（施工 I-70 中上）。
fn builtins_of(drivers: &[Mounted]) -> Vec<(Platform, Arc<dyn Builtin>)> {
    drivers
        .iter()
        .flat_map(|one| {
            one.builtins
                .iter()
                .map(|builtin| (one.platform, Arc::clone(builtin)))
        })
        .collect()
}

mod answers;
mod builtins;
mod follow;
mod listed;

#[cfg(test)]
mod tests;
