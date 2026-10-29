//! 原来的三个配置项挪进会话默认值（施工 I-7 下二，项目主人 10-20 定设置页的会话默认值存成一张卡）：睡觉时间、群里的触发词、她看得到 QQ 号
//! 原来是 im 的配置项（`configured`，拼成 `60-settings.toml`），现在照会话默认值那张卡（`cards.json` 的 `defaults`）。
//!
//! 起来时看一次：配置里设了的、卡上那一格还没写的搬过去（存盘、换上）；设了的配置项不管搬没搬都去掉（经核心的 `config.set`：扩展改得了
//! 自己包编号开头的系统配置），以后只照卡。没认出来的不搬、不去掉（`Configured::move_into`）：照旧报，改对了下次起来再搬。清单里这三项留着、藏起来（`hidden`），老的配置照读。去不掉的记一行运行日志，下次起来再试。

use serde_json::{Value, json};

use super::Route;
use crate::TARGET;
use crate::core::{Gone, reason};

impl Route {
    /// 搬（模块的说明）。
    ///
    /// # Errors
    ///
    /// 核心断开。
    pub(super) async fn move_defaults(&mut self) -> Result<(), Gone> {
        let mut next = self.rules.cards().clone();
        let moved = self.rules.configured().move_into(&mut next.defaults);
        if moved.is_empty() {
            return Ok(());
        }
        if next != *self.rules.cards() {
            let (words, lang) = (self.rules.page_words().clone(), self.core.language.clone());
            if self.store(next, &words, &lang)["saved"] == false {
                return Ok(());
            }
        }
        let changes: Vec<Value> = moved
            .iter()
            .map(|name| json!({"key": format!("{}.{name}", crate::PREFIX), "unset": true}))
            .collect();
        let reply = self
            .core
            .call("config.set", json!({"layer": "system", "changes": changes}))
            .await?;
        match reason(&reply) {
            None => {
                tracing::info!(target: TARGET, count = moved.len(), "settings moved into chat defaults")
            }
            Some(why) => tracing::warn!(target: TARGET, why, "settings not cleared after moving"),
        }
        Ok(())
    }
}
