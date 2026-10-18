//! 连着的机器人号（施工 I-2 上从 `listen/bots.rs` 挪来，`onebot.md` 第一条「怎么走」第 2、10 条）：一个号一条连接，同一个号
//! 再连进来，新的顶掉旧的（旧的由驱动关掉）；回话照号找它现在的连接。后台页的 `status`、状态文件照这里说连着哪个号、是哪个
//! 实现（施工 O-16、O-18、O-28）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Map, Value, json};

use super::Link;

/// 号到连接。
#[derive(Debug, Default)]
pub struct Bots {
    links: Mutex<HashMap<i64, Arc<dyn Link>>>,
}

impl Bots {
    /// 号 `bot` 现在用 `link`：原来有一条的，交回它（调的一方关掉它）。
    pub fn insert(&self, bot: i64, link: Arc<dyn Link>) -> Option<Arc<dyn Link>> {
        self.lock().insert(bot, link)
    }

    /// 号 `bot` 现在的连接。
    pub fn get(&self, bot: i64) -> Option<Arc<dyn Link>> {
        self.lock().get(&bot).cloned()
    }

    /// 连着的号里最小的那一个和它的连接：`status` 只说一个（`onebot.md` 第一条「后台页」第 2 条）。没有连着的是空的。
    pub fn first(&self) -> Option<(i64, Arc<dyn Link>)> {
        self.lock()
            .iter()
            .min_by_key(|(bot, _)| **bot)
            .map(|(bot, link)| (*bot, Arc::clone(link)))
    }

    /// 连着的每一个号和它的连接，照号排（施工 O-58 上：`cards.get` 问每个号在的群、好友）。
    pub fn all(&self) -> Vec<(i64, Arc<dyn Link>)> {
        let mut all: Vec<(i64, Arc<dyn Link>)> = self
            .lock()
            .iter()
            .map(|(bot, link)| (*bot, Arc::clone(link)))
            .collect();
        all.sort_by_key(|(bot, _)| *bot);
        all
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
    pub fn remove(&self, bot: i64, link: &Arc<dyn Link>) {
        let mut links = self.lock();
        if links.get(&bot).is_some_and(|now| Arc::ptr_eq(now, link)) {
            links.remove(&bot);
        }
    }

    /// 锁里不 `await`、不会崩；真崩了，表照样能用。
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<i64, Arc<dyn Link>>> {
        self.links.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
