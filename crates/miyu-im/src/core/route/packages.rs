//! 挂接点要的软件包列表（施工 O-43 上，`onebot.md` 第一条「挂接点」第 1 条）：起来时订阅核心的 `packages`（回应同
//! `package.list`），之后推来的 `package.changed` 照换那一项，掉了队（`resync`）重新订阅。谁挂在哪记在 `crate::core::hooks`。
//! 核心拒了订阅的记一行，照没有挂接点跑；请求写出去就接着办，不等回应，桥照常起来。
//!
//! im 的插件（施工 I-8 上，`crate::plugins`）同样起来时读一遍、核心推 `content.changed` 时再读，在小任务里读，不等。

use std::sync::Arc;

use serde_json::{Value, json};

use super::Route;
use crate::TARGET;
use crate::core::{Gone, reason};
use crate::plugins::load;

impl Route {
    /// 订阅软件包列表，照回应记下谁挂在哪：请求当场写出去（跟别的请求的先后一定），等回应交给一个小任务（`chores`），跟核心的
    /// 那一头不等它、照常往下办（核心的替身不答也不卡）；之后推来的 `package.changed` 照旧从这一头的读进来。核心拒了的记一行。
    ///
    /// 回应和推送走两条路：回应晚于一条推送办完的，整份换上的是订阅那一刻的样子，那一个包等它下一次变了再推来就对了。
    ///
    /// # Errors
    ///
    /// 写不出去（核心断开）。
    pub(super) async fn follow_packages(&mut self) -> Result<(), Gone> {
        let answer = self
            .core
            .caller()
            .send("subscribe", json!({"stream": "packages"}))
            .await?;
        let hooks = Arc::clone(&self.hooks);
        self.chores.spawn(async move {
            // 等不到的（核心断开）：跟核心的那一头会读到头、桥停下，这里没有别的要做。
            if let Ok(reply) = answer.wait().await {
                match reason(&reply) {
                    None => hooks.learn(&reply["result"]["packages"]),
                    Some(why) => tracing::warn!(target: TARGET, why, "packages not followed"),
                }
            }
        });
        Ok(())
    }

    /// 读一遍 im 的插件（`crate::plugins::load`），读完交给挂接点：`content.list` 当场写出去（跟别的请求的先后一定），等回应、读
    /// 文件在小任务（`chores`）里，跟核心的那一头不等它（核心的替身不答也不卡）。
    ///
    /// # Errors
    ///
    /// 写不出去（核心断开）。
    pub(super) async fn reload_plugins(&mut self) -> Result<(), Gone> {
        let caller = self.core.caller();
        let listed = caller.send("content.list", json!({})).await?;
        let hooks = Arc::clone(&self.hooks);
        self.chores.spawn(async move {
            // 等的时候核心断开：跟核心的那一头会读到头、桥停下，这里没有别的要做。
            if let Ok(plugins) = load(&caller, listed).await {
                hooks.set_plugins(plugins);
            }
        });
        Ok(())
    }

    /// 核心推来的一条 `pushed` 是不是软件包列表的：`package.changed` 照换那一项，`resync`（`packages`）重新订阅。是的交回真。
    ///
    /// # Errors
    ///
    /// 重新订阅时写不出去（核心断开）。
    pub(super) async fn package_pushed(&mut self, pushed: &Value) -> Result<bool, Gone> {
        let params = &pushed["params"];
        match pushed["method"].as_str() {
            Some("package.changed") => {
                if let Some(package) = params["package"].as_str() {
                    self.hooks.changed(package, &params["entry"]);
                }
                Ok(true)
            }
            // 给 im 带内容的包装、卸、升级了（施工 I-8 上）：插件再读一遍。
            Some("content.changed") => {
                self.reload_plugins().await?;
                Ok(true)
            }
            Some("resync") if params["stream"] == "packages" => {
                self.follow_packages().await?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
