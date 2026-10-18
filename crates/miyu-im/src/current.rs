//! 桥手里最新的配置（`onebot.md` 第一条「怎么走」第 1、2 条，施工 O-20）：握手交来的那一份，核心推来 `extension.config` 就照它
//! 换上，都不读盘。驱动的那几项（施工 I-3：OneBot 的端口、令牌）挑出来经 `watch` 交给驱动（[`Current::driver`]），驱动拿它
//! 做什么宿主不管；群的几项（施工 O-41 补）也在这里一份：后台页的 `status`、状态文件照它说哪几项没认出来（场所规则用的那一份
//! 在 `crate::rules::Venues`）。

use std::sync::{Mutex, MutexGuard, PoisonError};

use serde_json::{Map, Value};
use tokio::sync::watch;

use crate::rules::Configured;
use crate::settings::{driver_config, merge_driver};

/// 桥手里最新的配置。驱动、后台页的方法、状态文件共用一份。
pub(crate) struct Current {
    /// 驱动的配置项的名字（`Driver::settings`）。
    names: &'static [&'static str],
    /// 驱动的配置项推来 `null` 的照它（清单的默认值）。
    defaults: Map<String, Value>,
    /// 驱动的那几项最新的值，照名字。
    driver: watch::Sender<Map<String, Value>>,
    /// 群的几项最新的那一份（施工 O-41 补）。
    configured: Mutex<Configured>,
}

impl Current {
    /// 手里先拿着握手交来的 `config`：驱动的那几项 `names` 挑出来（没交的照 `defaults`），群的几项是 `configured`。
    pub(crate) fn new(
        config: &Value,
        (names, defaults): (&'static [&'static str], Map<String, Value>),
        configured: Configured,
    ) -> Current {
        let (driver, _) = watch::channel(driver_config(config, names, &defaults));
        Current {
            names,
            defaults,
            driver,
            configured: Mutex::new(configured),
        }
    }

    /// 驱动的那几项：拿着的随时读到最新的，变了收到通知。
    pub(crate) fn driver(&self) -> watch::Receiver<Map<String, Value>> {
        self.driver.subscribe()
    }

    /// 驱动的配置项 `name` 最新的值；没有的是空的。
    pub(crate) fn driver_value(&self, name: &str) -> Option<Value> {
        self.driver.borrow().get(name).cloned()
    }

    /// 驱动的配置项推来 `null` 的照的默认值。
    pub(crate) fn driver_default(&self, name: &str) -> Option<Value> {
        self.defaults.get(name).cloned()
    }

    /// 没认出来的配置项（施工 O-41 补，`Configured::misread`）。
    pub(crate) fn misread(&self) -> Vec<String> {
        lock(&self.configured).misread()
    }

    /// 推来的变化 `keys` 合进来（`merge_driver`、`Configured::change`）：驱动的那几项变了的通知驱动。交回没认出来的配置项变了
    /// 没有：变了的由调的一方重写状态文件。
    pub(crate) fn change(&self, keys: &Map<String, Value>) -> bool {
        self.driver
            .send_if_modified(|driver| merge_driver(driver, keys, self.names, &self.defaults));
        let mut configured = lock(&self.configured);
        let misread = configured.misread();
        let regrouped = configured.change(keys);
        regrouped && configured.misread() != misread
    }
}

/// 锁里不 `await`、不会崩；真崩了，里面的照样能用。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
