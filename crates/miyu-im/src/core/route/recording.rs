//! 记聊天记录（设计 04，施工 C-1 上，`crate::records`）：人说的照记进场所会话的同一处记一份，她说的照发出去了记，撤回、禁言照
//! 平台推来的记。只交给写的线程，不等、不影响回不回。

use miyu_chat::Venue;
use miyu_client::ids::ExternalId;
use serde_json::{Map, Value, json};

use super::Route;
use crate::chat::{Media, Posted};
use crate::records::{self, Happening, Said, Write};

impl Route {
    /// 场所 `venue` 里 `sender` 说的 `posted`，正文 `text`（@ 已经写成名字、合并转发写了占位）。
    pub(super) fn record_heard(
        &self,
        venue: &Venue,
        sender: &ExternalId,
        posted: &Posted,
        text: &str,
    ) {
        let mentions = posted
            .segments
            .at()
            .iter()
            .filter_map(|user| self.platform.person(*user).ok())
            .map(|person| person.as_str().to_string())
            .collect();
        let said = Said {
            venue: venue.id().as_str().to_string(),
            msg: posted.message_id.to_string(),
            sender: sender.as_str().to_string(),
            name: posted.name.clone(),
            at: records::at(posted.time),
            text: text.to_string(),
            reply_to: posted.segments.reply_to.clone(),
            mentions,
            media: posted.segments.media.iter().map(media).collect(),
            mine: false,
        };
        self.records.write(Write::Said(Box::new(said)));
    }

    /// 她（机器人号 `bot`）在场所 `venue` 发出去的一条，平台编号 `msg`，正文 `text`；`image` 的是一张图。
    pub(super) fn record_mine(&self, venue: &str, bot: i64, (msg, text): (i64, &str), image: bool) {
        let Ok(sender) = self.platform.person(bot) else {
            return;
        };
        let said = Said {
            venue: venue.to_string(),
            msg: msg.to_string(),
            sender: sender.as_str().to_string(),
            name: None,
            at: records::now(),
            text: text.to_string(),
            reply_to: None,
            mentions: Vec::new(),
            media: if image {
                vec![json!({"kind": "image"})]
            } else {
                Vec::new()
            },
            mine: true,
        };
        self.records.write(Write::Said(Box::new(said)));
    }

    /// 场所 `venue` 里的一件事 `kind`：对谁 `target`、谁做的 `actor`（号），禁言几秒、撤回的那一条的编号。
    pub(super) fn record_happening(
        &self,
        (venue, kind): (&str, &'static str),
        (target, actor): (Option<i64>, Option<i64>),
        (seconds, msg): (Option<u64>, Option<i64>),
    ) {
        let person = |number: Option<i64>| {
            number
                .and_then(|number| self.platform.person(number).ok())
                .map(|person| person.as_str().to_string())
        };
        let happening = Happening {
            venue: venue.to_string(),
            at: records::now(),
            kind,
            target: person(target),
            actor: person(actor),
            seconds: seconds.and_then(|seconds| i64::try_from(seconds).ok()),
            msg: msg.map(|msg| msg.to_string()),
        };
        self.records.write(Write::Happened(happening));
    }
}

/// 带的一样东西记成的那一格：种类、平台编号，有的才写名字、大小（不存内容）。
fn media(media: &Media) -> Value {
    let mut one = Map::new();
    one.insert("kind".into(), json!(media.kind.as_str()));
    one.insert("id".into(), json!(media.id));
    if let Some(name) = &media.name {
        one.insert("name".into(), json!(name));
    }
    if let Some(size) = media.size {
        one.insert("size".into(), json!(size));
    }
    Value::Object(one)
}
