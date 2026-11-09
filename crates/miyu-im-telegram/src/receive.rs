//! 收到的一条消息认成宿主的一件事（施工 I-183 中下，设计 07 第 3.5 节「照 AstrBot 踩过的坑」）：
//!
//! - 私聊（`private`）交 `Event::Private`，群（`group`、`supergroup`）交 `Event::Group`；频道、别的机器人发的不收（机器人本来就看不到
//!   别的机器人说的）。改过的消息第一版不收（长轮询只要 `message`）。
//! - 名字照 `first_name` 加 `last_name`（AstrBot #6657 只取用户名）；名字、用户名、群名、话题记进 [`Known`]。
//! - 正文照 `entities`（偏移是 UTF-16 的）切：`@她的用户名`、`text_mention` 是 @ 了谁；`/命令@她` 也算叫她，`/命令@别的机器人` 不算
//!   （AstrBot #10240）；@ 了不认得用户名的人照原样留字。
//! - 回复她的消息算叫她（设计 07：隐私模式开着时这是群里少数收得到的；QQ 那边引用她不算叫她，I-20，所以在这里补一个 @ 她，宿主不动）。
//! - 话题群：话题里的消息带着话题的「建话题」那条当引用，不算引用；记下话题，回话回在那里。
//! - 带的东西照 QQ 那边的种类给占位（图、贴纸、语音、视频、文件），第一版不取内容（取随 I-185）。

use serde_json::Value;

use miyu_im::chat::{Event, Id, Media, MediaKind, Piece, Posted, Segments};

use crate::bot::full_name;
use crate::known::{Known, Person};

/// 机器人自己：号、用户名（不带 `@`）。
pub struct Me<'a> {
    /// 号。
    pub bot: &'a Id,
    /// 用户名。
    pub username: &'a str,
}

/// 消息 `message`（`Update.message`）交给宿主的那件事；不收的是空的。
pub fn event(me: &Me<'_>, message: &Value, known: &Known) -> Option<Event> {
    let chat = &message["chat"];
    let chat_id = id_of(&chat["id"])?;
    let from = &message["from"];
    if from["is_bot"] == true {
        return None;
    }
    let user = id_of(&from["id"])?;
    let name = full_name(from);
    let username = from["username"].as_str().map(str::to_string);
    known.set_person(
        user.as_str(),
        Person {
            name: name.clone(),
            username,
        },
    );
    let thread = (message["is_topic_message"] == true)
        .then(|| message["message_thread_id"].as_i64())
        .flatten();
    known.set_thread(chat_id.as_str(), thread);
    if let Some(title) = chat["title"].as_str() {
        known.set_title(chat_id.as_str(), title);
    }
    let mut segments = segments(me, message, known);
    let reply = &message["reply_to_message"];
    let to_topic = thread.is_some() && reply["message_id"].as_i64() == thread;
    if !reply.is_null() && !to_topic {
        segments.reply_to = reply["message_id"].as_i64().map(|id| id.to_string());
        if id_of(&reply["from"]["id"]).as_ref() == Some(me.bot) && !segments.at().contains(me.bot) {
            segments.pieces.push(Piece::At(me.bot.clone()));
        }
    }
    let text = segments
        .pieces
        .iter()
        .filter_map(|piece| match piece {
            Piece::Text(text) => Some(text.as_str()),
            Piece::At(_) => None,
        })
        .collect();
    let posted = Posted {
        bot: me.bot.clone(),
        user,
        message_id: message["message_id"].as_i64()?,
        time: message["date"].as_i64().unwrap_or(0),
        name: (!name.is_empty()).then_some(name),
        rank: None,
        text,
        segments,
    };
    match chat["type"].as_str()? {
        "private" => Some(Event::Private(posted)),
        "group" | "supergroup" => Some(Event::Group {
            group: chat_id,
            posted,
        }),
        _ => None,
    }
}

/// 号：Telegram 给的整数写成字（群号是负数）。
fn id_of(value: &Value) -> Option<Id> {
    Id::new(&value.as_i64()?.to_string())
}

/// 正文（字或图片说明）照实体切成几块，再加带的东西。
fn segments(me: &Me<'_>, message: &Value, known: &Known) -> Segments {
    let (text, entities) = match message["text"].as_str() {
        Some(text) => (text, &message["entities"]),
        None => (
            message["caption"].as_str().unwrap_or_default(),
            &message["caption_entities"],
        ),
    };
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut pieces = Vec::new();
    let mut at = 0;
    for entity in entities.as_array().into_iter().flatten() {
        let (Some(offset), Some(length)) = (entity["offset"].as_u64(), entity["length"].as_u64())
        else {
            continue;
        };
        let (Ok(start), Ok(length)) = (usize::try_from(offset), usize::try_from(length)) else {
            continue;
        };
        let end = start.saturating_add(length).min(units.len());
        if start < at || start >= end {
            continue;
        }
        let Some(piece) = piece(
            me,
            entity,
            &String::from_utf16_lossy(&units[start..end]),
            known,
        ) else {
            continue;
        };
        push_text(&mut pieces, &units[at..start]);
        pieces.extend(piece);
        at = end;
    }
    push_text(&mut pieces, &units[at..]);
    Segments {
        pieces,
        media: media(message),
        ..Segments::default()
    }
}

/// 一段字（UTF-16）接在后面；空的不放。
fn push_text(pieces: &mut Vec<Piece>, units: &[u16]) {
    if !units.is_empty() {
        pieces.push(Piece::Text(String::from_utf16_lossy(units)));
    }
}

/// 一个实体认成的几块；照原样留字的是空的。
fn piece(me: &Me<'_>, entity: &Value, slice: &str, known: &Known) -> Option<Vec<Piece>> {
    match entity["type"].as_str()? {
        "mention" => {
            let name = slice.strip_prefix('@')?;
            name.eq_ignore_ascii_case(me.username)
                .then(|| vec![Piece::At(me.bot.clone())])
        }
        "text_mention" => {
            let user = &entity["user"];
            let id = id_of(&user["id"])?;
            let username = user["username"].as_str().map(str::to_string);
            known.set_person(
                id.as_str(),
                Person {
                    name: full_name(user),
                    username,
                },
            );
            Some(vec![Piece::At(id)])
        }
        "bot_command" => {
            let (command, target) = slice.split_once('@')?;
            target.eq_ignore_ascii_case(me.username).then(|| {
                vec![
                    Piece::Text(format!("{command} ")),
                    Piece::At(me.bot.clone()),
                ]
            })
        }
        _ => None,
    }
}

/// 带的东西：照 QQ 那边的种类给占位，第一版不取（`fetch` 空的）。
fn media(message: &Value) -> Vec<Media> {
    let one = |kind, id: &Value, name: Option<&str>, size: &Value| {
        Some(Media {
            kind,
            id: id.as_str().filter(|id| !id.is_empty())?.to_string(),
            name: name
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string),
            size: size.as_u64(),
            fetch: None,
            url: None,
        })
    };
    let found = if let Some(sizes) = message["photo"].as_array() {
        // 几种大小，最后一个最大。
        sizes.last().and_then(|photo| {
            one(
                MediaKind::Image,
                &photo["file_id"],
                None,
                &photo["file_size"],
            )
        })
    } else if message["sticker"].is_object() {
        let sticker = &message["sticker"];
        one(
            MediaKind::Sticker,
            &sticker["file_id"],
            sticker["emoji"].as_str(),
            &sticker["file_size"],
        )
    } else if message["voice"].is_object() {
        one(
            MediaKind::Voice,
            &message["voice"]["file_id"],
            None,
            &message["voice"]["file_size"],
        )
    } else if let Some(video) = ["video", "video_note", "animation"]
        .into_iter()
        .map(|key| &message[key])
        .find(|value| value.is_object())
    {
        one(
            MediaKind::Video,
            &video["file_id"],
            video["file_name"].as_str(),
            &video["file_size"],
        )
    } else if let Some(file) = ["document", "audio"]
        .into_iter()
        .map(|key| &message[key])
        .find(|value| value.is_object())
    {
        one(
            MediaKind::File,
            &file["file_id"],
            file["file_name"].as_str().or(file["title"].as_str()),
            &file["file_size"],
        )
    } else {
        None
    };
    found.into_iter().collect()
}

#[cfg(test)]
mod tests;
