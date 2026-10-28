//! 告诉核心每个场所开着哪些插件（施工 S-1 上c，核心 O-60 的 `venue.plugins`；项目主人 10-24 定：插件在一个场所关着时，它的工具
//! 不该出现在那个场所的工具表里，不然白耗 token）。核心只记在内存里，没说过的场所带 `plugin` 的工具一件都不给，所以：
//!
//! - 起来订阅完、每办完一件事（卡片、插件、驱动的配置变了都在里面），对一遍认得的每个场所（[`Route::sync_plugins`]）；
//! - 头一次找到一个场所的会话、第一轮以前，先说这一个（[`Route::tell_plugins`]，`session.rs`）。
//!
//! 开着哪些照 `Hooks::plugins_on`（场所卡片、插件声明的默认、驱动带的另看总开关）。和上一次说的一样的不再说；核心拒了的记一行、
//! 也记下（不然每办一件事都再发一次）。

use std::collections::BTreeMap;

use serde_json::json;

use super::Route;
use crate::TARGET;
use crate::chat::{Platform, To};
use crate::core::{Gone, reason};

impl Route {
    /// 平台 `platform` 上的场所 `venue`（发到 `to`）开着的插件和上一次告诉核心的不一样，就告诉一次。
    ///
    /// # Errors
    ///
    /// 核心断开。
    pub(super) async fn tell_plugins(
        &mut self,
        venue: &str,
        (platform, to): (Platform, To),
    ) -> Result<(), Gone> {
        let on = self.hooks.plugins_on(platform, to);
        if self.plugins_told.get(venue) == Some(&on) {
            return Ok(());
        }
        let reply = self
            .core
            .caller()
            .call("venue.plugins", json!({"venue": venue, "on": on}))
            .await?;
        match reason(&reply) {
            None => tracing::info!(target: TARGET, venue, on = on.join(","), "venue plugins told"),
            Some(why) => {
                tracing::warn!(target: TARGET, venue, reason = why, "venue plugins refused")
            }
        }
        self.plugins_told.insert(venue.to_string(), on);
        Ok(())
    }

    /// 认得的每个场所都对一遍（模块的说明）。
    ///
    /// # Errors
    ///
    /// 核心断开。
    pub(super) async fn sync_plugins(&mut self) -> Result<(), Gone> {
        let venues: BTreeMap<String, (Platform, To)> = self
            .peers
            .values()
            .map(|peer| (peer.venue.as_str().to_string(), (peer.platform, peer.to)))
            .collect();
        for (venue, at) in venues {
            self.tell_plugins(&venue, at).await?;
        }
        Ok(())
    }
}
