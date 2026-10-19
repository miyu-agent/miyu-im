//! 桥手里最新的配置（`onebot.md` 第一条「怎么走」第 1、2 条，施工 O-20）：握手交来的那一份，核心推来 `extension.config` 就照它
//! 换上，都不读盘。驱动的那几项（施工 I-3：OneBot 的端口、令牌）挑出来经 `watch` 交给驱动（[`Current::driver`]），驱动拿它
//! 做什么宿主不管；页面服务的那几项（施工 I-5 上：端口）一样挑出来交给页面服务（[`Current::site`]）；群的几项（施工 O-41 补）
//! 也在这里一份：后台页的 `status`、状态文件照它说哪几项没认出来（场所规则用的那一份在 `crate::rules::Venues`）。

use std::sync::{Mutex, MutexGuard, PoisonError};

use serde_json::{Map, Value};
use tokio::sync::watch;

use crate::rules::Configured;
use crate::settings::{SITE, driver_key, merge, pick, site_key};

/// 挑出来的一组配置项：名字、键怎么拼、推来 `null` 的照的默认值、最新的值（施工 I-5 上从驱动的那一份抽出来）。
struct Watched {
    /// 配置项的名字。
    names: &'static [&'static str],
    /// 名字拼成键。
    key: fn(&str) -> String,
    /// 推来 `null` 的照它（清单的默认值）。
    defaults: Map<String, Value>,
    /// 最新的值，照名字。
    values: watch::Sender<Map<String, Value>>,
}

impl Watched {
    /// 从握手交来的 `config` 里挑出 `names`（键照 `key` 拼，没交的照 `defaults`）。
    fn new(
        config: &Value,
        (names, key): (&'static [&'static str], fn(&str) -> String),
        defaults: Map<String, Value>,
    ) -> Watched {
        let (values, _) = watch::channel(pick(config, (names, key), &defaults));
        Watched {
            names,
            key,
            defaults,
            values,
        }
    }

    /// 推来的变化 `keys` 合进来：变了的通知拿着的。
    fn change(&self, keys: &Map<String, Value>) {
        self.values
            .send_if_modified(|values| merge(values, keys, (self.names, self.key), &self.defaults));
    }
}

/// 桥手里最新的配置。驱动、页面服务、后台页的方法、状态文件共用一份。
pub(crate) struct Current {
    /// 驱动的那几项（`Driver::settings`）。
    driver: Watched,
    /// 页面服务的那几项（[`SITE`]，施工 I-5 上）。
    site: Watched,
    /// 群的几项最新的那一份（施工 O-41 补）。
    configured: Mutex<Configured>,
}

impl Current {
    /// 手里先拿着握手交来的 `config`：驱动的那几项 `names` 挑出来（没交的照 `defaults`），页面服务的那几项照 `site` 的默认值
    /// 挑，群的几项是 `configured`。
    pub(crate) fn new(
        config: &Value,
        (names, defaults): (&'static [&'static str], Map<String, Value>),
        site: Map<String, Value>,
        configured: Configured,
    ) -> Current {
        Current {
            driver: Watched::new(config, (names, driver_key), defaults),
            site: Watched::new(config, (SITE, site_key), site),
            configured: Mutex::new(configured),
        }
    }

    /// 驱动的那几项：拿着的随时读到最新的，变了收到通知。
    pub(crate) fn driver(&self) -> watch::Receiver<Map<String, Value>> {
        self.driver.values.subscribe()
    }

    /// 驱动的配置项 `name` 最新的值；没有的是空的。
    pub(crate) fn driver_value(&self, name: &str) -> Option<Value> {
        self.driver.values.borrow().get(name).cloned()
    }

    /// 驱动的配置项推来 `null` 的照的默认值。
    pub(crate) fn driver_default(&self, name: &str) -> Option<Value> {
        self.driver.defaults.get(name).cloned()
    }

    /// 页面服务的那几项（施工 I-5 上）：同 [`Current::driver`]。
    pub(crate) fn site(&self) -> watch::Receiver<Map<String, Value>> {
        self.site.values.subscribe()
    }

    /// 没认出来的配置项（施工 O-41 补，`Configured::misread`）。
    pub(crate) fn misread(&self) -> Vec<String> {
        lock(&self.configured).misread()
    }

    /// 推来的变化 `keys` 合进来（`merge`、`Configured::change`）：驱动、页面服务的那几项变了的通知它们。交回没认出来的配置项
    /// 变了没有：变了的由调的一方重写状态文件。
    pub(crate) fn change(&self, keys: &Map<String, Value>) -> bool {
        self.driver.change(keys);
        self.site.change(keys);
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
