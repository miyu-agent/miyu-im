//! 连着的机器人号（施工 I-2 上从 `listen/bots.rs` 挪来，`onebot.md` 第一条「怎么走」第 2、10 条）：一个号一条连接，同一个号
//! 再连进来，新的顶掉旧的（旧的由驱动关掉）；回话照号找它现在的连接。后台页的 `status`、状态文件照这里说连着哪个号、是哪个
//! 实现（施工 O-16、O-18、O-28）；im 的页面照这里画连接卡，连上的时刻也记在这里（施工 I-6 上：「已连 3 小时」）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use serde_json::{Map, Value, json};

use super::{Id, Link};

/// 一个号现在的连接和它连上的时刻。
#[derive(Debug)]
struct Held {
    link: Arc<dyn Link>,
    since: SystemTime,
}

/// 号到连接；断开了的号记下什么时候断的（施工 I-6 中：页面上留着卡写「断开 4 分钟，在重连」），同一个号连回来就拿掉。
#[derive(Debug, Default)]
pub struct Bots {
    links: Mutex<HashMap<Id, Held>>,
    gone: Mutex<HashMap<Id, SystemTime>>,
}

impl Bots {
    /// 号 `bot` 现在用 `link`，这一刻连上：原来有一条的，交回它（调的一方关掉它）。
    pub fn insert(&self, bot: Id, link: Arc<dyn Link>) -> Option<Arc<dyn Link>> {
        let since = SystemTime::now();
        self.gone
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&bot);
        self.lock()
            .insert(bot, Held { link, since })
            .map(|held| held.link)
    }

    /// 号 `bot` 现在的连接。
    pub fn get(&self, bot: &Id) -> Option<Arc<dyn Link>> {
        self.lock().get(bot).map(|held| Arc::clone(&held.link))
    }

    /// 连着的号里最小的那一个和它的连接：`status` 只说一个（`onebot.md` 第一条「后台页」第 2 条）。没有连着的是空的。
    pub fn first(&self) -> Option<(Id, Arc<dyn Link>)> {
        self.lock()
            .iter()
            .min_by_key(|(bot, _)| *bot)
            .map(|(bot, held)| (bot.clone(), Arc::clone(&held.link)))
    }

    /// 经哪个号：`wanted` 写了、连着的照它；不然退回连着的号里最小的那一个（[`Bots::first`]，施工 I-70 上）。一个都没连着的是空的。
    pub fn pick(&self, wanted: Option<Id>) -> Option<Id> {
        wanted
            .filter(|bot| self.get(bot).is_some())
            .or_else(|| self.first().map(|(bot, _)| bot))
    }

    /// 连着的每一个号和它的连接，照号排（施工 O-58 上：`cards.get` 问每个号在的群、好友）。
    pub fn all(&self) -> Vec<(Id, Arc<dyn Link>)> {
        self.held()
            .into_iter()
            .map(|(bot, link, _)| (bot, link))
            .collect()
    }

    /// 连着的每一个号、它的连接、连上的时刻，照号排（施工 I-6 上：页面的连接卡）。
    pub fn held(&self) -> Vec<(Id, Arc<dyn Link>, SystemTime)> {
        let mut held: Vec<(Id, Arc<dyn Link>, SystemTime)> = self
            .lock()
            .iter()
            .map(|(bot, held)| (bot.clone(), Arc::clone(&held.link), held.since))
            .collect();
        held.sort_by(|(left, _, _), (right, _, _)| left.cmp(right));
        held
    }

    /// 连没连上、是哪个号、哪个实现：连着的号里最小的那一个，`connected`、`self_id`，问到了是哪个实现的再带
    /// `implementation`、`version`；没连着的只有 `connected: false`（`onebot.md` 第一条「后台页」第 2 条、「状态文件」）。
    pub fn napcat(&self) -> Value {
        let Some((bot, link)) = self.first() else {
            return json!({"connected": false});
        };
        let mut napcat = Map::new();
        napcat.insert("connected".into(), json!(true));
        napcat.insert("self_id".into(), json!(bot.to_string()));
        if let Some(implementation) = link.implementation() {
            napcat.insert("implementation".into(), json!(implementation.name));
            napcat.insert("version".into(), json!(implementation.version));
        }
        Value::Object(napcat)
    }

    /// 号 `bot` 的连接 `link` 断开了：号 `bot` 现在还是它的，拿掉；已经被顶掉的不动。
    pub fn remove(&self, bot: &Id, link: &Arc<dyn Link>) {
        let mut links = self.lock();
        if links
            .get(bot)
            .is_some_and(|now| Arc::ptr_eq(&now.link, link))
        {
            links.remove(bot);
            self.gone
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(bot.clone(), SystemTime::now());
        }
    }

    /// 断开了、还没连回来的号和断开的时刻，照号排（施工 I-6 中）。
    pub fn gone(&self) -> Vec<(Id, SystemTime)> {
        let mut gone: Vec<(Id, SystemTime)> = self
            .gone
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .map(|(bot, at)| (bot.clone(), *at))
            .collect();
        gone.sort_by(|(left, _), (right, _)| left.cmp(right));
        gone
    }

    /// 锁里不 `await`、不会崩；真崩了，表照样能用。
    /// 只有号 `bot` 的一份（施工 I-81 中：会话页一个连接只看它自己的群、好友）；没连着的是空的。
    pub fn only(&self, bot: &Id) -> Bots {
        let only = Bots::default();
        if let Some(held) = self.lock().get(bot) {
            only.lock().insert(
                bot.clone(),
                Held {
                    link: Arc::clone(&held.link),
                    since: held.since,
                },
            );
        }
        only
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<Id, Held>> {
        self.links.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
