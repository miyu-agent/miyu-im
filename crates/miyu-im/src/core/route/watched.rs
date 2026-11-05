//! 场所会话的事件交给实时日志的汇总（设计 10，施工 I-160 中，`crate::watch::hub`）：群的主线、支线、私聊都照场所交，支线照它的
//! 主线找场所。不是场所会话的（桥自己开的本机会话）不交。
//!
//! 收到的消息是桥自己交进会话的（`session.send`），核心不推回来：交成了以后照交的正文、场所的格和回应里的序号补一条（[`Route::sent_in`]）。

use serde_json::{Value, json};

use super::message::Message;
use super::{Route, applied};

impl Route {
    /// 会话 `session` 推来一条事件 `event`：照它的场所交给汇总；支线照主线的场所，块的编号照支线自己。
    pub(super) fn watched(&self, session: &str, event: &Value) {
        let main = self.main_of_lane(session);
        let main = main.as_deref().unwrap_or(session);
        // 新场所的第一条推来时会话可能还没登记：收到的消息自己带着场所（`by.venue`）。
        let venue = match self.peers.get(main) {
            Some(peer) => Some(peer.venue.as_str()),
            None => event["by"]["venue"].as_str(),
        };
        if let Some(venue) = venue {
            self.watch.heard(venue, session, event);
        }
    }

    /// 桥把收到的 `message` 交进了会话 `session`，核心回了 `reply`（`events` 是这一句的序号）：照它补一条收到的给汇总。
    pub(super) fn sent_in(&self, session: &str, message: &Message, reply: &Value) {
        let events = &reply["result"]["events"];
        let seq = events
            .as_array()
            .and_then(|all| all.last())
            .and_then(Value::as_u64)
            .or(events.as_u64());
        let at = applied::clock().map(|clock| clock.now.to_string());
        let event = json!({"seq": seq, "at": at, "kind": "message.user", "body": {
            "blocks": [{"type": "text", "text": message.text}], "venue": message.fields}});
        self.watch
            .heard(message.place.peer.venue.as_str(), session, &event);
    }
}
