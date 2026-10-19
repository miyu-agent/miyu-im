//! 场所会话的对话模型（施工 O-58 补三，设计 01 配置项「对话模型」）：卡片、场所规则的 `model` 经核心的 `session.configure` 钉到
//! 场所会话上（核心第 5 件：`model: null` 回到跟着配置的 `models.chat`）。
//!
//! - 第一次找到会话（`session`）：规则设了的钉上，等它钉好再交消息，第一轮就照它。没设的不碰：人在终端、网页里给这个会话换的
//!   模型照旧。
//! - 卡片存了（`cards.save`、两页改一张卡）：记着的每个场所会话比一比换卡以前、以后规则设的模型，变了的照新的钉，清掉的回到
//!   跟着配置；另起小任务，不等。记着的私聊都是套了规则的（白名单成员、照「其他私聊」接的陌生人）或者终端管理员（卡片管
//!   不到，换卡以前以后一样，不会钉）。
//! - 核心拒了（`unknown_model` 这类）记一行运行日志，会话照原来的模型。

use std::collections::HashMap;

use miyu_chat::Venue;
use miyu_client::ids::VenueId;
use serde_json::json;

use super::Route;
use crate::TARGET;
use crate::core::{Caller, reason};

impl Route {
    /// 记着的场所会话此刻照规则该钉的模型：会话编号 → 模型（规则没设的是空的）。
    pub(super) fn models(&mut self) -> HashMap<String, Option<String>> {
        let venues: Vec<(String, String)> = self
            .venues
            .iter()
            .map(|(venue, session)| (venue.clone(), session.clone()))
            .collect();
        let mut models = HashMap::new();
        for (venue, session) in venues {
            let Some(parsed) = VenueId::parse(&venue).ok().and_then(|id| Venue::parse(&id)) else {
                continue;
            };
            let applied = self.applied(&parsed);
            let model = super::applied::model(&applied).map(str::to_string);
            models.insert(session, model);
        }
        models
    }

    /// 卡片换了以后（`before` 是换以前 [`Route::models`] 的样子）：变了的照新的钉，清掉的回到跟着配置。另起小任务，不等。
    pub(super) fn repin(&mut self, before: &HashMap<String, Option<String>>) {
        for (session, model) in self.models() {
            if before.get(&session) == Some(&model) {
                continue;
            }
            let caller = self.core.caller();
            self.chores
                .spawn(async move { pin(&caller, &session, model.as_deref()).await });
        }
    }
}

/// 经 `session.configure` 给会话 `session` 钉模型 `model`（空的是回到跟着配置）。核心拒了的记一行运行日志；核心断了的不管
/// （跟核心的那一头自己会停）。
pub(super) async fn pin(caller: &Caller, session: &str, model: Option<&str>) {
    let params = json!({"session": session, "model": model});
    let Ok(reply) = caller.call("session.configure", params).await else {
        return;
    };
    match reason(&reply) {
        None => tracing::info!(target: TARGET, session, model, "venue model pinned"),
        Some(reason) => {
            tracing::warn!(target: TARGET, session, model, reason, "venue model not pinned");
        }
    }
}
