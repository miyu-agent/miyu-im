//! 发图、发文件（施工 I-185 中，设计 07 第 3.5 节）：宿主交的图（`Outgoing::Image`、长文画成的 `Picture`）照 `sendPhoto` 传，文件
//! （`Outgoing::Upload`）照 `sendDocument` 传，都是 `multipart/form-data`，和发字同一条队（先后照调的）。
//!
//! - 宿主交的是 `base64://…`；不是的照本机的路径读（驱动和 im 在同一台机器上）。
//! - Bot API 的上限：图 10 MB、文件 50 MB。超了 10 MB 的图改照文件发（Telegram 不压它），超了 50 MB 的不发，答为什么。
//! - 长文画成的图带 `lead` 的引用和 @：@ 写进图的说明（`caption`），引用照 `reply_parameters`。
//! - 表情包（`sticker`）也照图发：`sendSticker` 只收 512 px 以内的 WebP、PNG，宿主交的不一定合。
//!
//! 请求体自己拼（不为它多带依赖）：几格字，一格文件，分隔符照进程号和此刻的纳秒拼，碰不上正文的概率可以不管（正文里撞上了
//! Telegram 回读不懂，照没成答）。

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Map, Value};

/// `sendPhoto` 最大 10 MB。
pub const PHOTO_MOST: usize = 10 * 1024 * 1024;

/// `sendDocument` 最大 50 MB。
pub const DOCUMENT_MOST: usize = 50 * 1024 * 1024;

/// 要传的一份：照哪个方法、放在哪一格、叫什么、内容，图的说明（字和实体）。
#[derive(Debug, Clone)]
pub struct Upload {
    /// `sendPhoto` 或 `sendDocument`。
    pub method: &'static str,
    /// `photo` 或 `document`。
    pub field: &'static str,
    /// 文件名。
    pub name: String,
    /// 内容。
    pub bytes: Vec<u8>,
    /// 图的说明：字和它的实体（@ 人）；没有的是空的。
    pub caption: Option<(String, Vec<Value>)>,
}

impl Upload {
    /// 一张图（`file` 照模块的说明读）：超了 10 MB 的改照文件发，超了 50 MB 的交回为什么。
    ///
    /// # Errors
    ///
    /// 读不出、太大：为什么（英文短句，答宿主的 `failed` 照它）。
    pub fn image(file: &str, caption: Option<(String, Vec<Value>)>) -> Result<Upload, String> {
        let bytes = read(file)?;
        let photo = bytes.len() <= PHOTO_MOST;
        if !photo && bytes.len() > DOCUMENT_MOST {
            return Err(too_large());
        }
        Ok(Upload {
            method: if photo { "sendPhoto" } else { "sendDocument" },
            field: if photo { "photo" } else { "document" },
            name: "image.png".to_string(),
            bytes,
            caption,
        })
    }

    /// 一份文件，叫 `name`。
    ///
    /// # Errors
    ///
    /// 同 [`Upload::image`]。
    pub fn document(file: &str, name: &str) -> Result<Upload, String> {
        let bytes = read(file)?;
        if bytes.len() > DOCUMENT_MOST {
            return Err(too_large());
        }
        let name = name.trim();
        Ok(Upload {
            method: "sendDocument",
            field: "document",
            name: if name.is_empty() { "file" } else { name }.to_string(),
            bytes,
            caption: None,
        })
    }
}

/// 太大的那一句。
fn too_large() -> String {
    "file is larger than 50 MB".to_string()
}

/// 读宿主交的 `file`：`base64://…` 解开，别的照本机的路径读。
fn read(file: &str) -> Result<Vec<u8>, String> {
    match file.strip_prefix("base64://") {
        Some(encoded) => STANDARD
            .decode(encoded)
            .map_err(|_| "not valid base64".to_string()),
        None => std::fs::read(file.strip_prefix("file://").unwrap_or(file))
            .map_err(|error| format!("cannot read the file: {}", error.kind())),
    }
}

/// 拼请求体：几格字 `fields`（字照原样，别的写成 JSON），一格文件 `upload`。交回 `Content-Type` 和正文。
pub fn form(fields: &Map<String, Value>, upload: &Upload) -> (String, Vec<u8>) {
    let boundary = format!(
        "miyu-im-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos())
    );
    let mut body = Vec::new();
    for (name, value) in fields {
        let text = match value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{text}\r\n"
            )
            .as_bytes(),
        );
    }
    // 文件名里的引号、换行换掉：只当名字看。
    let name: String = upload
        .name
        .chars()
        .map(|ch| {
            if matches!(ch, '"' | '\r' | '\n' | '\\') {
                '_'
            } else {
                ch
            }
        })
        .collect();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"{}\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n",
            upload.field
        )
        .as_bytes(),
    );
    body.extend_from_slice(&upload.bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

#[cfg(test)]
mod tests;
