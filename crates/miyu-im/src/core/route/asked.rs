//! 驱动带的插件请 im 替它问一次模型（施工 Q-7，`Event::Asked`，入群审批）：照那个场所的判断模型问（会话的旁路请求先照它：设计 01
//! 第一节第 20 条的「旁路请求模型」还没有，判断模型是同一档便宜的；没设的照 `models.chat`），另起任务经核心的 `model.call` 问，
//! 回的字交回去；核心拒了的交原因，断开了的交 `gone`。等多久由插件自己定。

use serde_json::{Value, json};

use super::Route;
use crate::TARGET;
use crate::chat::{Replier, To};
use crate::core::reason;

impl Route {
    /// 替驱动带的插件问一次模型（模块的说明）。
    pub(super) fn ask_model(&mut self, to: To, mut request: Value, reply: Replier) {
        let model = match to {
            To::Group(group) => self
                .platform
                .venue(miyu_chat::VenueKind::Group, group)
                .ok()
                .and_then(|venue| self.applied(&venue).params.judge.model),
            To::Private(_) => None,
        };
        if let Some(model) = model {
            request["model"] = json!(model);
        }
        let caller = self.core.caller();
        self.chores.spawn(async move {
            let answer = match caller.call("model.call", request).await {
                Ok(answered) => match reason(&answered) {
                    None => Ok(answered["result"]["text"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string()),
                    Some(why) => {
                        tracing::warn!(target: TARGET, why, "model call for a plugin refused");
                        Err(why.to_string())
                    }
                },
                Err(_) => Err("gone".to_string()),
            };
            reply.send(answer);
        });
    }
}
