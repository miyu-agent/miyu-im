//! 桥手里最新的配置（`onebot.md` 第一条「怎么走」第 1、2 条，施工 O-20）：握手交来的那一份，核心推来 `extension.config` 就照它
//! 换上。NapCat 的监听照它的令牌比，后台页的方法照它答，换端口照它的端口，都不读盘。群的七项（施工 O-41 补）也在这里一份：
//! 后台页的 `status`、状态文件照它说哪几项没认出来（场所规则用的那一份在 `crate::rules::Venues`）。
//!
//! 换了令牌，已经连着的 NapCat 不断：只有连进来的那一下比令牌。端口变了由 `serve` 另起一个任务换（`crate::running::rebind`，
//! [`Current::change`] 交回变没变）。

use std::sync::{Mutex, MutexGuard, PoisonError};

use serde_json::{Map, Value};

use miyu_client::settings::Secret;

use crate::TARGET;
use crate::rules::Configured;
use crate::settings::{Defaults, Settings};

/// 桥手里最新的配置。NapCat 的监听、后台页的方法、换端口共用一份。
pub(crate) struct Current {
    /// 推来 `null` 的端口照它（清单的默认值）。
    defaults: Defaults,
    /// 最新的那一份。
    settings: Mutex<Settings>,
    /// 群的七项最新的那一份（施工 O-41 补）。
    configured: Mutex<Configured>,
}

/// 推来的变化改了什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Changed {
    /// NapCat 的端口变了：调的一方换（`crate::running::rebind`）。
    pub(crate) port: bool,
    /// 没认出来的配置项变了：调的一方重写状态文件（施工 O-41 补）。
    pub(crate) misread: bool,
}

impl Current {
    /// 手里先拿着握手交来的 `settings`、群的七项 `configured`；推来 `null` 的端口照 `defaults`。
    pub(crate) fn new(settings: Settings, configured: Configured, defaults: Defaults) -> Current {
        Current {
            defaults,
            settings: Mutex::new(settings),
            configured: Mutex::new(configured),
        }
    }

    /// 没认出来的配置项（施工 O-41 补，`Configured::misread`）。
    pub(crate) fn misread(&self) -> Vec<String> {
        lock(&self.configured).misread()
    }

    /// 最新的那一份。
    pub(crate) fn settings(&self) -> Settings {
        lock(&self.settings).clone()
    }

    /// 最新的令牌；没有的是空的。
    pub(crate) fn token(&self) -> Option<Secret> {
        lock(&self.settings).token.clone()
    }

    /// 推来的变化 `keys` 合进来（[`Settings::change`]、`Configured::change`）。令牌换了记一行，不记值。交回端口、没认出来的
    /// 配置项变了没有：变了的由调的一方换端口（`crate::running::rebind`）、重写状态文件。
    pub(crate) fn change(&self, keys: &Map<String, Value>) -> Changed {
        let mut settings = lock(&self.settings);
        let before = settings.clone();
        settings.change(keys, &self.defaults);
        if settings.token != before.token {
            tracing::info!(target: TARGET, "token changed");
        }
        let mut configured = lock(&self.configured);
        let misread = configured.misread();
        let regrouped = configured.change(keys);
        Changed {
            port: settings.port != before.port,
            misread: regrouped && configured.misread() != misread,
        }
    }
}

/// 锁里不 `await`、不会崩；真崩了，里面的照样能用。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
