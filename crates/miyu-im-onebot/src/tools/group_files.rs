//! 群文件（施工 I-12，设计 18 第十一节「群文件」「下载文件」）：`qq_group_files {folder?, file?}`，只给群。不写 `file` 的列一层：
//! 最外一层，或者 `folder` 那个文件夹（编号照列出来的）；文件夹写 `[folder] <名字> (id=<编号>, <几个> files)`，文件写
//! `<名字> (id=<编号>, <多大>, by <谁传的>)`，`; ` 隔开，最多列 `members_shown` 样。写了 `file` 的（编号照列出来的）问 QQ 要下载地址、
//! 下下来存进这个会话工作区的 `qq-files/`，回路径（同头像、取文件）。

use serde_json::Value;

use miyu_im::chat::{GroupFiles, To};
use miyu_im::toolset::{Call, NAME, Planned, clean};

use super::QqTools;
use super::lookup::{Undone, fetched, said, undone};

/// 一个大小，照人的单位：`512 B`、`1.2 KB`、`3.4 MB`、`1.0 GB`。
pub(crate) fn size(bytes: u64) -> String {
    let units = [("GB", 1u64 << 30), ("MB", 1 << 20), ("KB", 1 << 10)];
    for (unit, scale) in units {
        if bytes >= scale {
            #[allow(clippy::cast_precision_loss)]
            let value = bytes as f64 / scale as f64;
            return format!("{value:.1} {unit}");
        }
    }
    format!("{bytes} B")
}

/// 一层群文件列成一段（纯逻辑）：交回（列的几样、一共几样、那一段）。名字洗过，最多 `shown` 样，文件夹在前。
pub(crate) fn listed(files: &GroupFiles, shown: usize) -> (usize, usize, String) {
    let text = |name: &str| clean(name, NAME).unwrap_or_default();
    let mut all: Vec<String> = files
        .folders
        .iter()
        .map(|(id, name, count)| format!("[folder] {} (id={id}, {count} files)", text(name)))
        .collect();
    all.extend(files.files.iter().map(|file| {
        let by = file
            .uploader
            .as_deref()
            .map(text)
            .filter(|by| !by.is_empty())
            .map(|by| format!(", by {by}"))
            .unwrap_or_default();
        format!(
            "{} (id={}, {}{by})",
            text(&file.name),
            file.id,
            size(file.size)
        )
    }));
    let total = all.len();
    let list: Vec<String> = all.into_iter().take(shown).collect();
    (list.len(), total, list.join("; "))
}

/// 下下来的群文件存成什么名字（纯逻辑）：下载地址里带了 `fname=` 的照它（先解开 `%XX`，再洗过、去掉路径），不然
/// `group-file-<编号>`。
pub(crate) fn saved_name(url: &str, file: &str) -> String {
    let named = url
        .split(['?', '&'])
        .find_map(|part| part.strip_prefix("fname="))
        .map(decoded)
        .map(|name| name.rsplit(['/', '\\']).next().unwrap_or(&name).to_string())
        .and_then(|name| clean(&name, NAME))
        .filter(|name| !name.trim().is_empty() && name != "." && name != "..");
    named.unwrap_or_else(|| {
        let id: String = file
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .take(32)
            .collect();
        format!("group-file-{id}")
    })
}

/// 解开地址里的 `%XX`（平台给的文件名是这样写的）；不成对的 `%` 照原样，解出来不是 UTF-8 的那几个字节换成替换字符。
fn decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = bytes
            .get(at + 1..at + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match (bytes[at], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                at += 3;
            }
            (byte, _) => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `qq_group_files` 的一次调用（模块的说明）。私聊里调到的照不认识的工具答（`venues` 不给，照说不会）。
pub(super) fn group_files(tools: &QqTools, call: Call) -> Planned {
    let To::Group(group) = call.to else {
        return Planned::Now(call.answers.say("unknown", &[("name", &call.tool)], true));
    };
    let (shown, wait) = (tools.members, tools.wait);
    Planned::later(async move {
        let done = match call.args["file"].as_str().filter(|file| !file.is_empty()) {
            Some(file) => download(&call, group, file, wait).await,
            None => listing(&call, group, shown).await,
        };
        done.unwrap_or_else(|undone| said(&call, undone))
    })
}

/// 列一层。
async fn listing(call: &Call, group: i64, shown: usize) -> Result<Value, Undone> {
    let Some(link) = &call.link else {
        return Err(("unreachable", None));
    };
    let folder = call.args["folder"]
        .as_str()
        .filter(|folder| !folder.is_empty());
    let files = link.group_files(group, folder).await.map_err(undone)?;
    let (count, total, list) = listed(&files, shown);
    if count == 0 {
        return Ok(call.answers.say("no-group-files", &[], false));
    }
    let (count, total) = (count.to_string(), total.to_string());
    Ok(call.answers.say(
        "group-files",
        &[("shown", &count), ("total", &total), ("list", &list)],
        false,
    ))
}

/// 下一个文件存进工作区。
async fn download(
    call: &Call,
    group: i64,
    file: &str,
    wait: std::time::Duration,
) -> Result<Value, Undone> {
    let Some(link) = &call.link else {
        return Err(("unreachable", None));
    };
    let url = link.group_file_url(group, file).await.map_err(undone)?;
    let name = saved_name(&url, file);
    let path = fetched(&url, &name, (&call.cwd, std::env::home_dir()), wait).await?;
    Ok(call.answers.say("saved", &[("path", &path)], false))
}

#[cfg(test)]
mod tests;
