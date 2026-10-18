//! 认消息段（`onebot.md` 第一条「群消息」第 2 条，施工 O-22）：消息是段的数组时一段一段认成正文（字、@、占位）、引用、
//! @全体、带的东西；是字符串（CQ 码）时照第 6 条只读字（「施工时定的」第 62 条）。
//!
//! 只认、不洗：名字、编号合不合核心的写法（长短、控制字符）由交给核心的一方照 `venues.md`「场所的格」洗
//! （`core/route/fields.rs`）。不下载，带的东西只记平台的编号、大小和怎么取（懒下载，18 第五节；取的一方在
//! `core/route/pictures.rs`、`fetching.rs`，施工 O-33）。合并转发正文里照旧写占位，另记下它的编号和段里内联的消息，交给
//! 展开的一方（`core/route/forwards.rs`，施工 O-34）。

use serde_json::Value;

use super::{number, text_of};
pub use miyu_im::chat::{CARD, FORWARD, Fetch, Forwarded, Media, MediaKind, Piece, Segments};

/// 带的东西的平台编号从这几格取，照先后，头一个有的：NapCat 的图、语音、视频、文件是 `file_id` 或 `file`，小黄脸是 `id`，
/// 商城表情是 `emoji_id`。
const IDS: [&str; 4] = ["file_id", "file", "id", "emoji_id"];

/// 文件的名字从这几格取，照先后，头一个有的。
const FILE_NAMES: [&str; 3] = ["name", "file_name", "file"];

/// 认一段到 `segments` 里：`kind` 是段的 `type`，`data` 是它的 `data`。不认识的不记。
fn take(segments: &mut Segments, kind: &str, data: &Value) {
    match kind {
        "text" => {
            if let Some(text) = data["text"].as_str() {
                segments.pieces.push(Piece::Text(text.to_string()));
            }
        }
        "at" if data["qq"] == "all" => segments.mentions_all = true,
        "at" => {
            if let Some(user) = number(&data["qq"]) {
                segments.pieces.push(Piece::At(user));
            }
        }
        "reply" => {
            if segments.reply_to.is_none() {
                segments.reply_to = id(&data["id"]);
            }
        }
        "forward" => {
            segments.pieces.push(Piece::Text(FORWARD.to_string()));
            segments.forwards.push(Forwarded {
                id: id(&data["id"]),
                content: data["content"]
                    .as_array()
                    .map(|messages| messages.iter().map(super::node).collect()),
            });
        }
        "json" | "xml" => segments.pieces.push(Piece::Text(CARD.to_string())),
        _ => segments.media.extend(media(kind, data)),
    }
}

/// 认一条消息的 `message`：段的数组一段一段认；字符串（CQ 码）照第 6 条只读字（[`text_of`]）；别的什么都没有。
pub fn segments(message: &Value) -> Segments {
    let Value::Array(all) = message else {
        let text = text_of(message);
        let pieces = match text.is_empty() {
            true => Vec::new(),
            false => vec![Piece::Text(text)],
        };
        return Segments {
            pieces,
            ..Segments::default()
        };
    };
    let mut segments = Segments::default();
    for segment in all {
        take(
            &mut segments,
            segment["type"].as_str().unwrap_or_default(),
            &segment["data"],
        );
    }
    segments
}

/// 带的东西的一段：种类照段（图片里的表情包、商城表情记成表情），编号照 [`IDS`]，大小照 `file_size`，怎么取照段（施工 O-33）；
/// 不是带东西的段、一个编号都没有的不记。
fn media(kind: &str, data: &Value) -> Option<Media> {
    let image = Some(Fetch::Image);
    let (kind, name, fetch) = match kind {
        // NapCat 把商城表情发成 `image`，带 `emoji_id`，`summary` 是表情的字。
        "image" if !data["emoji_id"].is_null() => {
            (MediaKind::Sticker, words(&data["summary"]), image)
        }
        // `sub_type` 是 1 的是表情包；它的 `summary` 是「[动画表情]」这类，不是表情的字。
        "image" if number(&data["sub_type"]) == Some(1) => (MediaKind::Sticker, None, image),
        "image" => (MediaKind::Image, None, image),
        "mface" => (MediaKind::Sticker, words(&data["summary"]), None),
        "face" => (MediaKind::Sticker, words(&data["raw"]["faceText"]), None),
        "record" => (MediaKind::Voice, None, None),
        "video" => (MediaKind::Video, None, Some(Fetch::File)),
        "file" => (
            MediaKind::File,
            FILE_NAMES.iter().find_map(|key| words(&data[*key])),
            Some(Fetch::File),
        ),
        _ => return None,
    };
    let id = IDS.iter().find_map(|key| id(&data[*key]))?;
    Some(Media {
        kind,
        id,
        name,
        size: size(&data["file_size"]),
        fetch,
    })
}

/// 大小（施工 O-33）：非负整数，写成十进制的字也认（NapCat 照 NT 给的是字）；别的不认。
fn size(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

/// 一格编号：不空的字，或者整数写成十进制的字。
fn id(value: &Value) -> Option<String> {
    match value {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

/// 一格名字：去掉首尾空白不空的字，照原样。
fn words(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .map(str::to_string)
}
