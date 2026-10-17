//! 她发出去的图（施工 O-51，`onebot.md` 第一条「发图」；核心 X-19、设计 34 第二节第 5 条）：场所会话里一次工具结果带 `sent` 的
//! （表情包包的「发送表情包」这类，核心记进 `tool.result.sent`，不发给模型），一张一条发进 QQ：表情包（`sticker`）照 OneBot
//! 图片段带 `sub_type=1`、`subType=1`，QQ 当表情显示；别的照普通图片段。
//!
//! - 走出站队列（`sending`）：和她的字照事件的先后排（先说字再甩图、先甩图再说字都照她调的先后），被禁言着、没连着的排着，过期作废，
//!   同她的话。入队以前经 `blob.get` 把图读好、写成 `base64://…`（同展示，`shown::read`）；读不出的不入队，当没发出去办。
//! - 群里发出去了记 `venue.delivered`（正文空的、`images` 是这张图）；没发出去的（读不出、失败、过期、等不到）经核心的
//!   `session.note` 给那条线记一块 `unshown` 事实（模板同展示），她下一步知道没到。
//! - 补来的（订阅时从头补的）、这一轮不说话了的（`skip_reply`）不发：调的一方不交进来。

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use super::Route;
use super::sending::{Body, Image, Piece, What};
use super::shown::read;
use crate::TARGET;
use crate::core::{Gone, reason};
use crate::onebot::Lead;

/// 没发出去那一块事实的 `kind`（同展示）。
const UNSHOWN: &str = "unshown";

/// 一次工具结果 `sent` 里的一张：内容哈希、名字（没写的照 `alt`，再没有的写 `sticker`、`image`）、是不是表情包。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Sent {
    /// 内容哈希。
    pub(super) blob: String,
    /// 名字。
    pub(super) name: String,
    /// 是表情包。
    pub(super) sticker: bool,
}

/// 一次工具结果 `raw`（会话事件原样）里她发出去的图（纯逻辑）：不是工具结果的、没有的是空的；`blob` 不是字的那一张不要。
pub(super) fn sent_in(raw: &Value) -> Vec<Sent> {
    if raw["kind"] != "tool.result" {
        return Vec::new();
    }
    raw["body"]["sent"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|one| {
            let sticker = one["sticker"] == true;
            let name = one["name"]
                .as_str()
                .or_else(|| one["alt"].as_str())
                .map_or_else(
                    || (if sticker { "sticker" } else { "image" }).to_string(),
                    str::to_string,
                );
            Some(Sent {
                blob: one["blob"].as_str()?.to_string(),
                name,
                sticker,
            })
        })
        .collect()
}

impl Route {
    /// 会话 `session` 那条线 `line`（主线、支线、私聊）推来的一次工具结果 `raw` 里她发出去的图：一张张读好、入队（见模块开头）。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn stickers(
        &mut self,
        (session, line): (&str, &str),
        raw: &Value,
    ) -> Result<(), Gone> {
        let sent = sent_in(raw);
        if sent.is_empty() {
            return Ok(());
        }
        let turn = raw["turn"].as_u64().unwrap_or_default();
        for one in sent {
            let mut image = Image {
                line: line.to_string(),
                turn,
                blob: one.blob,
                name: one.name,
                sticker: one.sticker,
                file: String::new(),
            };
            match read(&self.core.caller(), Some(line), &image.blob).await {
                Ok(bytes) => image.file = format!("base64://{}", STANDARD.encode(bytes)),
                Err(why) => {
                    tracing::warn!(target: TARGET, session, why, "image not sent");
                    self.unsent_image(&image, ("unreadable", &why)).await?;
                    continue;
                }
            }
            let piece = Piece {
                what: What::Image(Box::new(image)),
                text: String::new(),
                lead: Lead::default(),
                body: Body::Text,
            };
            self.enqueue(session, None, piece).await?;
        }
        Ok(())
    }

    /// 她发出去的一张图 `image` 没到（为什么 `why`、原话 `detail`）：照展示的模板写一块 `unshown`，经核心的 `session.note` 记进
    /// 工具结果出现在的那条线。核心拒了的（封存了的支线这类）记一行，不再试。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn unsent_image(
        &mut self,
        image: &Image,
        (why, detail): (&str, &str),
    ) -> Result<(), Gone> {
        let fields = BTreeMap::from([
            ("why", why),
            ("detail", detail),
            ("name", image.name.as_str()),
        ]);
        let text = match self.rules.unshown().render(&fields) {
            Ok(text) => text,
            Err(error) => {
                // 照说不会：起来时查过模板只要这三个字段（`rules/facts.rs`）。
                tracing::warn!(target: TARGET, error = %error, "unshown not written");
                return Ok(());
            }
        };
        let facts = json!([{"kind": UNSHOWN, "text": text}]);
        let reply = self
            .core
            .call(
                "session.note",
                json!({"session": image.line, "facts": facts}),
            )
            .await?;
        if let Some(reason) = reason(&reply) {
            tracing::warn!(target: TARGET, line = image.line, reason, "note refused");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
