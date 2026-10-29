//! 消息记录页现取带的东西（施工 C-1 中下补，设计 04 第一节第 1 条：「页面上现取现显示、不落盘，取不到的说取不到了，可能已经失效」）：
//! `records.media {venue, msg, id}` → `{kind, type, name, data}`。
//!
//! 库里只记了占位和平台编号。照连着的机器人号一个个试：取回那一条（`get_msg` 这类，平台无关的 [`message`]），核对是这个场所的
//! （群里是这个群的、私聊里是对方发的或她发给对方的），在它带的东西里找编号是 `id` 的那一样，照它的取法拿到字节（[`take`]；群
//! 文件照下载地址），交回 base64、媒体类型、名字。最多 [`LIMIT`] 字节。
//!
//! 拒的原因：参数写错 `bad_params`；没有连着的机器人 `not_connected`；那一条、那一样找不到、平台不给、过期了 `gone`（页面说取不到了，
//! 可能已经失效）；取不了的种类（语音这类）`not_fetchable`；超过上限 `too_large`。

use std::sync::Arc;

use base64::Engine;
use serde_json::{Value, json};

use miyu_chat::Venue;
use miyu_client::ids::VenueId;

use super::Site;
use crate::chat::{Fetch, Fetched, Id, Link, Source, To};
use crate::media::{Missed, bytes, image_extension, message, take};

/// 页面上一次最多拿多少字节。
const LIMIT: usize = 16 * 1024 * 1024;

/// 拒的原因和原话。
type Refused = (String, String);

/// 只有原因的拒。
fn refused(why: &str) -> Refused {
    (why.to_string(), String::new())
}

/// 取回来的那一条在不在场所 `to` 里（纯逻辑）：群里是这个群的，私聊里是对方发的或机器人号 `bot` 发给对方的。
fn owned(fetched: &Fetched, to: &To, bot: &Id) -> bool {
    match to {
        To::Group(group) => fetched.group.as_ref() == Some(group),
        To::Private(user) => {
            fetched.private
                && (fetched.user.as_ref() == Some(user) || fetched.user.as_ref() == Some(bot))
        }
    }
}

/// 字节的媒体类型（纯逻辑）：认得的图写图的，别的是 `application/octet-stream`。
fn media_type(bytes: &[u8]) -> &'static str {
    match image_extension(bytes) {
        Some("jpg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        _ => "application/octet-stream",
    }
}

/// `records.media`（模块的说明）。
pub(super) async fn media(site: &Arc<Site>, params: &Value) -> Result<Value, Refused> {
    let venue = params["venue"]
        .as_str()
        .ok_or_else(|| refused("bad_params"))?;
    let msg = params["msg"]
        .as_str()
        .filter(|msg| !msg.is_empty())
        .ok_or_else(|| refused("bad_params"))?;
    let id = params["id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| refused("bad_params"))?;
    // 照场所编号的平台找驱动（施工 I-85 上）。
    let venue = VenueId::parse(venue)
        .ok()
        .and_then(|venue| Venue::parse(&venue))
        .ok_or_else(|| refused("bad_params"))?;
    let station = site
        .running
        .station(Some(venue.platform()))
        .ok_or_else(|| refused("bad_params"))?;
    let to = station
        .platform
        .to_of(&venue)
        .ok_or_else(|| refused("bad_params"))?;
    let held = station.bots.held();
    if held.is_empty() {
        return Err(refused("not_connected"));
    }
    let wait = site.running.tuning.fetch();
    let mut last = refused("gone");
    for (bot, link, _) in held {
        match fetched((&bot, link.as_ref()), (&to, msg, id), wait).await {
            Ok(found) => return Ok(found),
            Err(why) => last = why,
        }
    }
    Err(last)
}

/// 经机器人号 `bot` 的连接 `link` 取场所 `to` 里编号 `msg` 那一条带的编号 `id` 那一样，最多等 `wait`。
async fn fetched(
    (bot, link): (&Id, &dyn Link),
    (to, msg, id): (&To, &str, &str),
    wait: std::time::Duration,
) -> Result<Value, Refused> {
    let gone = |_| refused("gone");
    let found = message(link, to, msg).await.map_err(gone)?;
    if !owned(&found, to, bot) {
        return Err(refused("gone"));
    }
    let item = found
        .segments
        .media
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| refused("gone"))?;
    let how = item.fetch.ok_or_else(|| refused("not_fetchable"))?;
    if item.size.is_some_and(|size| size > LIMIT as u64) {
        return Err(refused("too_large"));
    }
    let taken = match (how, to) {
        // 群里的文件照群文件的下载地址取（同她取，施工 I-16）；拿不到的再照平台的取法试。
        (Fetch::File, To::Group(group)) => match link.group_file_url(group, &item.id).await {
            Ok(url) => bytes(&[Source::Url(url)], wait, LIMIT)
                .await
                .map(|bytes| (bytes, None))
                .map_err(Missed::Unfetched),
            Err(_) => take(link, how, &item.id, (wait, LIMIT)).await,
        },
        _ => take(link, how, &item.id, (wait, LIMIT)).await,
    };
    let (data, named) = taken.map_err(|missed| match missed {
        Missed::Unfetched(why) if why.contains("MB") => refused("too_large"),
        _ => refused("gone"),
    })?;
    let name = item.name.clone().or(named);
    Ok(json!({
        "kind": item.kind.as_str(),
        "type": media_type(&data),
        "name": name,
        "data": base64::engine::general_purpose::STANDARD.encode(&data),
    }))
}

#[cfg(test)]
mod tests;
