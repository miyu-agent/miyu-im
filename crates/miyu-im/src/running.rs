//! 桥跑着的样子（`onebot.md` 第一条「在哪」、「施工时定的」第 168 条，施工 O-28 下从 WebUI 的 `Web` 挪出来）：连着的机器人号、
//! 桥手里最新的配置、驱动报的状态（施工 I-3：OneBot 的端口、有没有令牌）、桥自己的数。后台页的 `status`
//! （`crate::core::methods`）、状态文件（`crate::status_file`）照它说。

use std::sync::Arc;

use serde_json::{Map, Value, json};

use crate::chat::Bots;
use crate::chat::Platform;
use crate::current::Current;
use crate::driver::Reported;
use crate::site::Port;
use crate::tuning::Tuning;

/// 跑着的桥，几样共用。
pub(crate) struct Running {
    /// 桥自己的数：`status` 的 `path` 照它的 `paths`。
    pub(crate) tuning: Tuning,
    /// 连着的机器人号：`status`、状态文件照它说 NapCat 连没连上。
    pub(crate) bots: Arc<Bots>,
    /// 平台（施工 I-2 下：驱动给的）。
    pub(crate) platform: Platform,
    /// 桥手里最新的配置（和驱动共用，施工 O-20）。
    pub(crate) current: Arc<Current>,
    /// 驱动报的状态（施工 I-3）。
    pub(crate) reported: Arc<Reported>,
    /// 页面服务实际听的端口（施工 I-5 上）。
    pub(crate) page: Arc<Port>,
}

impl Running {
    /// 后台页的 `status` 照它答的几格（施工 O-28 上，`onebot.md` 第一条「后台页」第 2 条）：`napcat`、驱动报的几格、`platform`，
    /// 照桥手里最新的，不读盘。
    ///
    /// - `napcat`：连着的号里最小的那一个，问到了是哪个实现的才带 `implementation`、`version`；没连着的只有
    ///   `connected: false`（`Bots::napcat`，状态文件也照它）。
    /// - 驱动报的：OneBot 是 `listen`（实际听的端口）、`token`（有令牌的 `set`，没有的 `none`；没写引用、引用取不到，核心都不交，
    ///   分不出，「施工时定的」第 41 条）。
    /// - `web`：页面服务实际听的端口（施工 I-5 上），没听上的不带。
    /// - `platform`：桥的平台名（驱动给的），白名单成员页照它拼 `qq:<号>`（第 151 条）。
    /// - `misread`：没认出来的配置项（施工 O-41 补，`Current::misread`），空的不带这一格。
    pub(crate) fn status(&self) -> Map<String, Value> {
        let mut status = Map::new();
        status.insert("napcat".into(), self.bots.napcat());
        status.extend(self.reported.get());
        status.insert("platform".into(), json!(self.platform.name()));
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
