//! 长文转成图（施工 O-59 上，`onebot.md`「场所卡片」的「长文」、「出站队列」）：她的一句话过了出站链以后，照这个场所的参数
//! `long_text` 是 `image`、`long_chars` 不是 0、全文超过 `long_chars` 个字的，经核心的 `package.call` 调渲染包
//! `miyu-render.markdown` 画成几页，每一页经 `blob.get` 读成 `base64://…`（不写会话：渲染包传的 blob 在管理员名下）。
//!
//! 桥这一头等着画完（同发表情包读图）：长图要和她前后的话守先后。等至多 [`WAIT`]；哪一步不成（没装、没开、拒了、超时、读不出）
//! 交回空的、记一行 `long text not drawn`，调的一方照拆段发。

use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use miyu_chat::{LongText, Params};
use miyu_client::ids::VenueId;
use serde_json::json;

use miyu_chat::Venue;

use super::Route;
use super::outbound::Passed;
use super::sending::{Image, Page, Piece, What};
use super::shown::read;
use crate::TARGET;
use crate::core::{Caller, Gone, reason};
use crate::onebot::Lead;

/// 渲染包的编号、方法。
const RENDER: (&str, &str) = ("miyu-render", "markdown");

/// 画加读最多等多久（渲染包自己限 30 秒，平常一页零点几秒）：超了照拆段发，不卡住别的群太久。
pub(super) const WAIT: Duration = Duration::from_secs(15);

/// 画好的一页：blob 的编号、NapCat 认的 `base64://…`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Drawn {
    pub(super) blob: String,
    pub(super) file: String,
}

/// 照参数 `params`，全文 `text` 算不算要画成图的长文。
pub(super) fn wants_image(params: &Params, text: &str) -> bool {
    params.long_text == LongText::Image
        && params.long_chars > 0
        && text.chars().count() > params.long_chars
}

/// 在场所 `venue` 把 `text` 画成几页、读好：不成的记一行、交回空的。`wait` 是最多等多久（正式跑是 [`WAIT`]）。
pub(super) async fn draw(
    caller: &Caller,
    venue: &VenueId,
    text: &str,
    wait: Duration,
) -> Option<Vec<Drawn>> {
    let chars = text.chars().count();
    match tokio::time::timeout(wait, pages(caller, text)).await {
        Ok(Ok(drawn)) => {
            tracing::info!(target: TARGET, venue = %venue, chars, pages = drawn.len(), "long text drawn");
            Some(drawn)
        }
        Ok(Err(why)) => {
            tracing::warn!(target: TARGET, venue = %venue, chars, why, "long text not drawn");
            None
        }
        Err(_) => {
            tracing::warn!(target: TARGET, venue = %venue, chars, why = "timeout", "long text not drawn");
            None
        }
    }
}

/// 调渲染包、读每一页。不成的交回原因：核心的原因码（`unknown_package`、`program_not_running`、`method_failed`……）、
/// `gone`（核心断开）、`no_pages`、`bad_reply`，读图的照 `shown::read`。
async fn pages(caller: &Caller, text: &str) -> Result<Vec<Drawn>, String> {
    let (package, method) = RENDER;
    let params = json!({"package": package, "method": method, "params": {"markdown": text}});
    let reply = caller
        .call("package.call", params)
        .await
        .map_err(|_| "gone".to_string())?;
    if let Some(why) = reason(&reply) {
        return Err(why.to_string());
    }
    let blobs: Vec<String> = reply["result"]["pages"]
        .as_array()
        .ok_or("bad_reply")?
        .iter()
        .map(|page| page["blob"].as_str().map(str::to_string))
        .collect::<Option<_>>()
        .ok_or("bad_reply")?;
    if blobs.is_empty() {
        return Err("no_pages".into());
    }
    let mut drawn = Vec::with_capacity(blobs.len());
    for blob in blobs {
        let bytes = read(caller, None, &blob).await?;
        let file = format!("base64://{}", STANDARD.encode(bytes));
        drawn.push(Drawn { blob, file });
    }
    Ok(drawn)
}

/// 这一页给 `unshown` 写的名字：第几页。
fn page_name(index: usize) -> String {
    format!("long-text-{}.png", index + 1)
}

impl Route {
    /// 她的话（`what` 是 [`What::Reply`]）过了链的 `passed` 是要画成图的长文、也画成了：一页一段照先后入队，第一页带全文和
    /// `lead`（拿走），交回真；不是、画不成的交回假，调的一方照拆段发。
    ///
    /// # Errors
    ///
    /// 入队时写不出去、核心断开。
    pub(super) async fn say_long(
        &mut self,
        session: &str,
        (passed, what): (&Passed, &What),
        lead: &mut Lead,
    ) -> Result<bool, Gone> {
        let What::Reply { line, turn, to } = what else {
            return Ok(false);
        };
        let Some(peer) = self.peers.get(session).cloned() else {
            return Ok(false);
        };
        let Some(venue) = Venue::parse(&peer.venue) else {
            return Ok(false);
        };
        if !wants_image(&self.applied(&venue).params, &passed.text) {
            return Ok(false);
        }
        let Some(drawn) = draw(&self.core.caller(), &peer.venue, &passed.text, WAIT).await else {
            return Ok(false);
        };
        for (index, page) in drawn.into_iter().enumerate() {
            let first = index == 0;
            let image = Image {
                line: line.clone(),
                turn: *turn,
                blob: page.blob,
                name: page_name(index),
                sticker: false,
                file: page.file,
            };
            let piece = Piece {
                what: What::Page(Box::new(Page {
                    image,
                    to: to.clone(),
                    first,
                })),
                text: if first {
                    passed.text.clone()
                } else {
                    String::new()
                },
                lead: std::mem::take(lead),
            };
            self.enqueue(session, None, piece).await?;
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
