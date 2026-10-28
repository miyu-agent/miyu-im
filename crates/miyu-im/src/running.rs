//! 桥跑着的样子（`onebot.md` 第一条「在哪」、「施工时定的」第 168 条，施工 O-28 下从 WebUI 的 `Web` 挪出来）：连着的机器人号、
//! 桥手里最新的配置、驱动报的状态（施工 I-3：OneBot 的端口、有没有令牌）、桥自己的数。后台页的 `status`
//! （`crate::core::methods`）、状态文件（`crate::status_file`）照它说。施工 I-70 中上起挂几个驱动，各自连着的号、报的状态一份
//! （[`Station`]）；只说一个的那几格照第一个驱动的（[`Running::first`]），`miyu im status`、后台页不用改。

use std::sync::Arc;

use serde_json::{Map, Value, json};

use crate::chat::Bots;
use crate::chat::Platform;
use crate::current::Current;
use crate::driver::Reported;
use crate::site::Port;
use crate::tuning::Tuning;

/// 一个驱动跑着的样子（施工 I-70 中上）。
pub(crate) struct Station {
    /// 平台（施工 I-2 下：驱动给的）。
    pub(crate) platform: Platform,
    /// 连着的机器人号：`status`、状态文件照它说 NapCat 连没连上。
    pub(crate) bots: Arc<Bots>,
    /// 驱动报的状态（施工 I-3）。
    pub(crate) reported: Arc<Reported>,
}

impl Station {
    /// 状态文件里这个驱动的那一格（`drivers.<平台>`）：同顶上的样子，连没连上（`napcat`，[`Bots::napcat`]）和驱动报的几格。
    pub(crate) fn status(&self) -> Map<String, Value> {
        let mut status = Map::new();
        status.insert("napcat".into(), self.bots.napcat());
        status.extend(self.reported.get());
        status
    }
}

/// 跑着的桥，几样共用。
pub(crate) struct Running {
    /// 桥自己的数：`status` 的 `path` 照它的 `paths`。
    pub(crate) tuning: Tuning,
    /// 挂着的驱动，照先后；起来时查过不空（`crate::serve`）。
    pub(crate) drivers: Vec<Station>,
    /// 桥手里最新的配置（和驱动共用，施工 O-20）。
    pub(crate) current: Arc<Current>,
    /// 页面服务实际听的端口（施工 I-5 上）。
    pub(crate) page: Arc<Port>,
    /// 桥起来的时刻（施工 I-14：统计页的运行时间）。
    pub(crate) since: std::time::SystemTime,
}

impl Running {
    /// 第一个驱动（`Platforms::first`）：只说一个的那几格、页面照它（页面分平台是 I-70 下的事）。
    ///
    /// # Panics
    ///
    /// 一个驱动都没有：`crate::serve` 起来时就拒了，是 bug。
    pub(crate) fn first(&self) -> &Station {
        self.drivers.first().expect("起来时查过至少一个驱动")
    }

    /// 后台页的 `status` 照它答的几格（施工 O-28 上，`onebot.md` 第一条「后台页」第 2 条）：`napcat`、驱动报的几格、`platform`
    /// 照第一个驱动的，`platforms` 是挂着的几个平台的名字（施工 I-70 中上），照桥手里最新的，不读盘。
    ///
    /// - `napcat`：连着的号里最小的那一个，问到了是哪个实现的才带 `implementation`、`version`；没连着的只有
    ///   `connected: false`（`Bots::napcat`，状态文件也照它）。
    /// - 驱动报的：OneBot 是 `listen`（实际听的端口）、`token`（有令牌的 `set`，没有的 `none`；没写引用、引用取不到，核心都不交，
    ///   分不出，「施工时定的」第 41 条）。
    /// - `web`：页面服务实际听的端口（施工 I-5 上），没听上的不带。
    /// - `platform`：第一个驱动的平台名（驱动给的），白名单成员页照它拼 `qq:<号>`（第 151 条）。
    /// - `platforms`：挂着的几个平台的名字，照先后（施工 I-70 中上）。
    /// - `misread`：没认出来的配置项（施工 O-41 补，`Current::misread`），空的不带这一格。
    pub(crate) fn status(&self) -> Map<String, Value> {
        let first = self.first();
        let mut status = first.status();
        status.insert("platform".into(), json!(first.platform.name()));
        let platforms: Vec<&str> = self.drivers.iter().map(|one| one.platform.name()).collect();
        status.insert("platforms".into(), json!(platforms));
        if let Some(port) = self.page.get() {
            status.insert("web".into(), json!(port));
        }
        let misread = self.current.misread();
        if !misread.is_empty() {
            status.insert("misread".into(), json!(misread));
        }
        status
    }
}
