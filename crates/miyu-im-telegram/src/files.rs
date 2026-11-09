//! 取带的东西（施工 I-185 上，设计 07 第 3.5 节「收图给她看」）：宿主照图片段的编号（`file_id`）问连接取（[`Link::fetch`]），这里
//! `getFile` 问到在哪、照这个连接的代理下回来，交 base64（宿主自己的下载不走连接的代理，地址里又带着令牌，不交出去）。
//!
//! - 机器人取文件最大 20 MB（Bot API 文档 `getFile`）：问到的大小超了的不下，下的时候读到上限就停。
//! - 文件名照 `file_path` 的最后一段（图是 `photos/file_12.jpg` 这样的，没什么用，宿主图只看字节）。
//!
//! [`Link::fetch`]: miyu_im::chat::Link::fetch

use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::json;

use miyu_im::chat::{CallError, Located, Source};

use crate::api::{Api, Failure};

/// 机器人取文件的上限：20 MB。
pub const MOST: usize = 20 * 1024 * 1024;

/// 编号是 `id` 的那一样在哪：问、下，最多各等 `wait`。
///
/// # Errors
///
/// Telegram 回了失败、超了上限（[`CallError::Failed`]），等不到（[`CallError::Timeout`]），连不上（[`CallError::Closed`]）。
pub async fn located(api: &Api, id: &str, wait: Duration) -> Result<Located, CallError> {
    let file = api
        .call("getFile", &json!({"file_id": id}), wait)
        .await
        .map_err(failed)?;
    let Some(path) = file["file_path"].as_str().filter(|path| !path.is_empty()) else {
        return Err(said("no file_path"));
    };
    if file["file_size"]
        .as_u64()
        .is_some_and(|size| usize::try_from(size).map_or(true, |size| size > MOST))
    {
        return Err(said("file is larger than 20 MB"));
    }
    let bytes = api.download(path, MOST, wait).await.map_err(failed)?;
    Ok(Located {
        sources: vec![Source::Base64(STANDARD.encode(bytes))],
        name: path
            .rsplit('/')
            .next()
            .filter(|name| !name.is_empty())
            .map(str::to_string),
    })
}

/// Telegram 的错换成宿主认的。
fn failed(failure: Failure) -> CallError {
    match failure {
        Failure::Failed { code, description } => CallError::Failed {
            said: format!("{code} {description}"),
            message: description,
        },
        Failure::Network(detail) if detail.starts_with("timeout") => CallError::Timeout,
        Failure::Network(detail) => said(&detail),
        _ => CallError::Closed,
    }
}

/// 没成：`detail` 是为什么。
fn said(detail: &str) -> CallError {
    CallError::Failed {
        said: detail.to_string(),
        message: detail.to_string(),
    }
}
