//! 她给人看的文件发进 QQ（施工 O-46，`onebot.md` 第一条「展示」；核心的 `show`，设计 35、`tools/show.md`）：场所会话里一次
//! 工具结果的效果有 `file.shown` 的，照媒体类型发：常见的四种图片发成图片段，别的（PDF、文本、svg……）传成文件。
//!
//! - 内容是核心的 blob：经 `blob.get` 一块块读（一块最多 512 KiB），带上它出现在的那个会话（`session`，核心照会话的属主找）。
//! - 交 NapCat 写成 `base64://…`（`image_to`、`upload_to`）。各起一个任务办（`chores`，同平台工具），跟核心的那一头不等。
//! - 发不了的（读不出、她被禁言着、没连着、NapCat 回失败、等不到）经核心的 `session.note` 给那个会话记一块 `unshown` 事实，
//!   她下一步看得到：工具结果已经说了「给人看了」，这一块说其实没到。不改那次工具结果，不另发一句。
//! - 补来的（订阅时从头补的）、这一轮不说话了的（`skip_reply`）不发：调的一方不交进来。

use std::collections::BTreeMap;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use miyu_client::protocol::Template;
use serde_json::{Value, json};

use super::{Peer, Route, applied};
use crate::TARGET;
use crate::chat::Bots;
use crate::chat::CallError;
use crate::chat::Outgoing;
use crate::core::{Caller, reason};

/// 发成图片段的媒体类型：QQ 画得出的四种；别的图（svg 这类）传成文件。
const IMAGES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];

/// `blob.get` 一块读多少字节：它的上限。
const CHUNK: u64 = 512 * 1024;

/// 没发出去那一块事实的 `kind`。
const UNSHOWN: &str = "unshown";

/// 一个 `file.shown` 里桥要的几格。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Shown {
    /// 内容的哈希。
    blob: String,
    /// 文件名。
    name: String,
    /// 媒体类型。
    media_type: String,
}

impl Shown {
    /// 读一项效果：是 `file.shown`、`blob`、`name`、`media_type` 都是字的才是。
    fn read(effect: &Value) -> Option<Shown> {
        if effect["kind"] != "file.shown" {
            return None;
        }
        Some(Shown {
            blob: effect["blob"].as_str()?.to_string(),
            name: effect["name"].as_str()?.to_string(),
            media_type: effect["media_type"].as_str()?.to_string(),
        })
    }

    /// 发成图片段（不是的传成文件）。
    fn is_image(&self) -> bool {
        IMAGES.contains(&self.media_type.as_str())
    }
}

/// 一次工具结果 `raw`（会话事件原样）里给人看的文件：不是工具结果的、没有的是空的。
pub(super) fn shown_in(raw: &Value) -> Vec<Shown> {
    if raw["kind"] != "tool.result" {
        return Vec::new();
    }
    raw["body"]["effects"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Shown::read)
        .collect()
}

/// 交给任务办的一份。
struct Job {
    /// 主线（群会话、私聊会话）：发到哪照它，`unshown` 记进支线失败（封存了）时改记这里。
    session: String,
    /// 工具结果出现在的那条线（主线、支线）：读 blob、记 `unshown` 照它。
    line: String,
    /// 发到哪。
    peer: Peer,
    /// 这一份。
    file: Shown,
    /// 她这时被禁言着：不试，直接记。
    muted: bool,
    /// 机器人号的连接。
    bots: Arc<Bots>,
    /// 读 blob、记事实。
    caller: Caller,
    /// 「文件没发出去」的模板。
    template: Template,
}

impl Route {
    /// 会话 `session` 那条线 `line`（主线、支线、私聊）推来的一次工具结果 `raw` 里给人看的文件：每一份各起一个任务发（见模块
    /// 开头）。不知道发到哪的记一行、不发。
    pub(super) fn show(&mut self, (session, line): (&str, &str), raw: &Value) {
        let files = shown_in(raw);
        if files.is_empty() {
            return;
        }
        let Some(peer) = self.peers.get(session).cloned() else {
            tracing::warn!(target: TARGET, session, why = "unknown_peer", count = files.len(), "file not shown");
            return;
        };
        let muted = applied::clock().is_some_and(|clock| {
            self.groups
                .get(session)
                .is_some_and(|group| group.muted(clock.now).is_some())
        });
        for file in files {
            let job = Job {
                session: session.to_string(),
                line: line.to_string(),
                peer: peer.clone(),
                file,
                muted,
                bots: self.platforms.bots(peer.platform),
                caller: self.core.caller(),
                template: self.rules.unshown().clone(),
            };
            self.chores.spawn(send(job));
        }
    }
}

/// 办一份：发成了记一行；没成的记一行、记一块 `unshown` 事实。
async fn send(job: Job) {
    let venue = job.peer.venue.as_str().to_string();
    let kind = if job.file.is_image() { "image" } else { "file" };
    match deliver(&job).await {
        Ok(()) => tracing::info!(target: TARGET, venue, kind, "file shown"),
        Err((why, detail)) => {
            tracing::warn!(target: TARGET, venue, kind, why, "file not shown");
            note(&job, why, &detail).await;
        }
    }
}

/// 读出内容、交 NapCat。没成的交回为什么（`muted`、`unreachable`、`unreadable`、`failed`、`unanswered`）和原话。
async fn deliver(job: &Job) -> Result<(), (&'static str, String)> {
    if job.muted {
        return Err(("muted", String::new()));
    }
    let Some(link) = job.bots.get(&job.peer.bot) else {
        return Err(("unreachable", String::new()));
    };
    let bytes = read(&job.caller, Some(&job.line), &job.file.blob)
        .await
        .map_err(|why| ("unreadable", why))?;
    let file = format!("base64://{}", STANDARD.encode(bytes));
    let outgoing = match job.file.is_image() {
        true => Outgoing::Image {
            file: &file,
            sticker: false,
        },
        false => Outgoing::Upload {
            file: &file,
            name: &job.file.name,
        },
    };
    match link.send(job.peer.to.clone(), outgoing).await {
        Ok(_) => Ok(()),
        Err(CallError::Failed { said, .. }) => Err(("failed", said)),
        Err(CallError::Timeout) => Err(("unanswered", String::new())),
        Err(CallError::Closed) => Err(("unreachable", String::new())),
        Err(CallError::Unsupported) => Err(("failed", "unsupported".to_string())),
    }
}

/// 经 `blob.get` 把会话 `session` 里出现过的 blob `blob` 一块块读完。读不成的交回原因（核心的原因码、`gone`、写法不对）。表情包
/// 也照它读（施工 O-51，`stickers`）；长文画成的图（施工 O-59 上，`long`）不写会话：渲染包传的在管理员名下。
pub(super) async fn read(
    caller: &Caller,
    session: Option<&str>,
    blob: &str,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    loop {
        let mut params = json!({"blob": blob, "offset": bytes.len(), "length": CHUNK});
        if let Some(session) = session {
            params["session"] = json!(session);
        }
        let reply = caller
            .call("blob.get", params)
            .await
            .map_err(|_| "gone".to_string())?;
        if let Some(why) = reason(&reply) {
            return Err(why.to_string());
        }
        let chunk = reply["result"]["data"]
            .as_str()
            .and_then(|data| STANDARD.decode(data).ok())
            .ok_or_else(|| "bad_reply".to_string())?;
        let size = reply["result"]["size"].as_u64().unwrap_or(0);
        let ended = chunk.is_empty();
        bytes.extend(chunk);
        if ended || bytes.len() as u64 >= size {
            return Ok(bytes);
        }
    }
}

/// 照模板写一块 `unshown`（为什么 `why`、原话 `detail`、文件名），经核心的 `session.note` 记进工具结果出现在的那条线；那条线是
/// 封存了的支线的（`session_sealed`）改记主线。核心拒了的记一行，不再试。
async fn note(job: &Job, why: &str, detail: &str) {
    let fields = BTreeMap::from([
        ("why", why),
        ("detail", detail),
        ("name", job.file.name.as_str()),
    ]);
    let Ok(text) = job.template.render(&fields) else {
        // 照说不会：起来时查过模板只要这三个字段（`rules/facts.rs`）。
        tracing::warn!(target: TARGET, "unshown not written");
        return;
    };
    let facts = json!([{"kind": UNSHOWN, "text": text}]);
    let mut reply = job
        .caller
        .call("session.note", json!({"session": job.line, "facts": facts}))
        .await;
    if job.line != job.session
        && reply
            .as_ref()
            .is_ok_and(|reply| reason(reply) == Some("session_sealed"))
    {
        reply = job
            .caller
            .call(
                "session.note",
                json!({"session": job.session, "facts": facts}),
            )
            .await;
    }
    match reply.as_ref().map(reason) {
        Ok(None) => tracing::info!(target: TARGET, why, "unshown noted"),
        Ok(Some(reason)) => tracing::warn!(target: TARGET, reason, "note refused"),
        // 核心断开：跟核心的那一头会读到头、桥停下。
        Err(_) => {}
    }
}

#[cfg(test)]
mod tests;
