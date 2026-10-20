//! 平台工具的调用的那一头（施工 O-31 起，`onebot.md` 第一条「平台工具（一）」第 2 条）：读的一头把要会话、投影、机器人号的连接的
//! `tool.call` 照推送交到这里：施工 T-1 到 T-4 起工具都是驱动交的（`driven`），交给它；别的照不认识的工具答。

use serde_json::Value;

use super::Route;
use crate::core::Gone;

impl Route {
    /// 核心转来的一次平台工具的调用 `pushed`（`tool.call`）：分给各自那一段（模块的说明）。
    ///
    /// # Errors
    ///
    /// 收这个会话留着的推送时写不出去、核心断开。
    pub(super) async fn tool_called(&mut self, pushed: &Value) -> Result<(), Gone> {
        let params = &pushed["params"];
        let tool = params["tool"].as_str().unwrap_or_default();
        if let Some(driven) = self.tools.driven(tool) {
            return self.driven_called(pushed, driven).await;
        }
        self.core
            .answerer()
            .answer(&pushed["id"], self.tools.call(tool));
        Ok(())
    }

    /// 把会话 `session` 留着的推送收进来：群的进投影（同「群里怎么叫她」第 3 条），私聊的照「怎么走」第 9 条那条路（记下最近一条
    /// 的引用）。
    pub(super) async fn gather(&mut self, session: &str) -> Result<(), Gone> {
        if self.groups.contains_key(session) {
            return self.catch_up(session).await;
        }
        for pushed in self.core.take_events(session) {
            self.say_privately(session, &pushed["params"]["event"])
                .await?;
        }
        Ok(())
    }
}
