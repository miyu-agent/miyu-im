//! 核心推来的一条怎么分（从 `route.rs` 挪出来，施工 O-43 下：那一份行数到了上限）：配置变了、平台工具的调用、软件包列表、给别的
//! 包调的方法、群会话和私聊会话的事件、掉了队的再订阅。

use serde_json::Value;

use super::{Projection, Route};
use crate::TARGET;
use crate::core::Gone;
use crate::core::methods::SEND;
use crate::settings::{recall_receipts, recall_receipts_key, whitelist, whitelist_key};

impl Route {
    /// 核心推来的一条：配置变了的（`extension.config`）交给 `serve`（施工 O-20），群的七项交给场所规则（施工 O-41），白名单
    /// 成员在里面的换上（施工 O-23），私聊找过的会话都忘掉（施工 O-27：删了的人下一条照新的白名单再找，就不接了；终端管理员的
    /// 找回来还是那一个，再订阅一次不重）；群会话的
    /// 事件收进投影、她新说的话发回群里（施工 O-23）；私聊会话的事件交给 `say_privately`：她的回话发回去（第 10 条），这一轮
    /// 不说话了的不发（施工 O-26，要看 `turn.ended`）；掉了队、会话停了的（`resync`）再订阅一次，群的照收到的最后一条接着补。
    /// 读的一头交来的平台工具的调用（`tool.call`，施工 O-31）交给 `acting`。
    pub(super) async fn pushed(&mut self, pushed: Value) -> Result<(), Gone> {
        let params = &pushed["params"];
        if pushed["method"] == "extension.config" {
            let Some(keys) = params["keys"].as_object() else {
                return Ok(());
            };
            // 群的七项（施工 O-41）：变了的当场重读，下一条消息照新的。
            self.rules.configure(keys);
            if let Some(value) = keys.get(&whitelist_key()) {
                self.whitelist = whitelist(value);
                let groups = &self.groups;
                self.venues
                    .retain(|_, session| groups.contains_key(session));
                tracing::info!(target: TARGET, count = self.whitelist.len(), "whitelist changed");
            }
            // 撤回回执的命令（施工 O-47）：下一条回执照新的。
            if let Some(value) = keys.get(&recall_receipts_key()) {
                self.recalled = recall_receipts(value, &self.recall_default);
                tracing::info!(target: TARGET, count = self.recalled.len(), "recall receipts changed");
            }
            if self.configured.send(keys.clone()).is_err() {
                // `serve` 不收了：桥在停，没有别处可交。
            }
            return Ok(());
        }
        if pushed["method"] == "tool.call" {
            return self.tool_called(&pushed).await;
        }
        // 软件包列表变了、订阅掉了队（施工 O-43 上，`packages`）。
        if self.package_pushed(&pushed).await? {
            return Ok(());
        }
        if pushed["method"] == "method.call" {
            // 给别的包调的方法（施工 O-42，`exports`）：读的一头只把那几个交过来；`qq.send` 要找对方的会话（施工 O-56，`outreach`）。
            if pushed["params"]["method"] == SEND {
                return self.send_exported(&pushed).await;
            }
            // 后台页的场所卡片（施工 O-58 上，`cards`）。
            if self.cards_called(&pushed) {
                return Ok(());
            }
            self.exported(&pushed);
            return Ok(());
        }
        let Some(session) = params["session"].as_str() else {
            return Ok(());
        };
        // 支线推来的走支线那条路，不当成私聊（施工 O-36，`lanes`）。
        if let Some(main) = self.main_of_lane(session) {
            let lane = session.to_string();
            return self.lane_heard(&main, &lane, &pushed).await;
        }
        let group = self.groups.get(session).map(Projection::last);
        match (pushed["method"].as_str(), group) {
            (Some("event"), Some(_)) => {
                let session = session.to_string();
                self.heard(&session, &pushed).await?;
            }
            (Some("resync"), Some(last)) if params["stream"] == "events" => {
                let session = session.to_string();
                self.follow(&session, last).await?;
            }
            (Some("event"), None) => {
                let session = session.to_string();
                self.say_privately(&session, &params["event"]).await?;
            }
            (Some("resync"), None) if params["stream"] == "events" => {
                let session = session.to_string();
                self.subscribe(&session, None).await?;
            }
            _ => {}
        }
        Ok(())
    }
}
