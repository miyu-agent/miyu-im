//! 群文件（施工 I-12，设计 18 第十一节「群文件」「下载文件」）：`qq_group_files {folder?, file?}`，只给群。不写 `file` 的列一层：
//! 最外一层，或者 `folder` 那个文件夹（编号照列出来的）；文件夹写 `[folder] <名字> (id=<编号>, <几个> files)`，文件写
//! `<名字> (id=<编号>, <多大>, by <谁传的>)`，`; ` 隔开，最多列 `members_shown` 样。写了 `file` 的（编号照列出来的）问 QQ 要下载地址、
//! 下下来存进这个会话工作区的 `qq-files/`，回路径（同头像、取文件）。
//!
//! 施工 I-204（项目主人 11-10：群文件永久空间满了传不上去，要能让她清）：`delete` 写一个文件编号的删掉那个文件。只听管理的人：
//! 这一轮是终端管理员、卡片上「允许使用管理员命令的用户」叫的才删，别人叫的答 `managers-only`、不动（程序拦，不靠她自觉）。QQ
//! 那头她得是群主、管理员，或者文件是她传的，不然 QQ 拒，照它说的答。

use serde_json::Value;

use miyu_im::chat::{GroupFiles, Id, To};
use miyu_im::toolset::{Call, NAME, Planned, UNLIMITED, clean};

use super::QqTools;
use super::args::{bad_args, group_only, optional_text};
use super::lookup::{Undone, fetched, said, undone};

/// 一个大小，照人的单位（施工 I-185 下下挪进宿主，传文件同用）。
pub(crate) use miyu_im::toolset::common::send_file::size;

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

/// 下下来的群文件存成什么名字（纯逻辑）：列表里这个文件的名字 `listed`（施工 I-16：SnowLuma 的下载地址里 `fname=` 写的是文件
/// 编号）；没有的照下载地址里的 `fname=`（先解开 `%XX`；就是编号的不算）；都没有的 `group-file-<编号>`。名字都洗过、去掉路径。
pub(crate) fn saved_name(listed: Option<&str>, url: &str, file: &str) -> String {
    let bare = |id: &str| id.trim_matches('/').to_string();
    let from_url = url
        .split(['?', '&'])
        .find_map(|part| part.strip_prefix("fname="))
        .map(decoded)
        .filter(|name| bare(name) != bare(file));
    let named = listed
        .map(str::to_string)
        .into_iter()
        .chain(from_url)
        .find_map(|name| plain(&name));
    named.unwrap_or_else(|| {
        let id: String = file
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .take(32)
            .collect();
        format!("group-file-{id}")
    })
}

/// 一个名字去掉路径、洗过；剩下空的、`.`、`..` 的不算。
fn plain(name: &str) -> Option<String> {
    let last = name.rsplit(['/', '\\']).next().unwrap_or(name);
    clean(last, NAME).filter(|name| !name.trim().is_empty() && name != "." && name != "..")
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

/// `qq_group_files` 的一次调用（模块的说明）。私聊里调到的答只给群（`venues` 不给，照说不会），参数写错的说哪个（施工 I-17）。
pub(super) fn group_files(tools: &QqTools, call: Call) -> Planned {
    let To::Group(group) = call.to.clone() else {
        return group_only(&call);
    };
    // 施工 I-17：写错的说哪个参数。
    for key in ["folder", "file", "delete"] {
        let bad = match key {
            "folder" => "folder must be a folder id",
            "delete" => "delete must be a file id",
            _ => "file must be a file id",
        };
        if let Err(bad) = optional_text(&call.args, key, bad) {
            return bad_args(&call, bad);
        }
    }
    let (shown, wait) = (tools.members, tools.wait);
    let text = |key: &str| {
        let written = call.args[key].as_str().filter(|text| !text.is_empty());
        written.map(str::to_string)
    };
    let (delete, file) = (text("delete"), text("file"));
    // 删只听管理的人（施工 I-204）：当场挡下，不问 QQ。
    if delete.is_some() && !call.admin && !call.manager {
        return Planned::Now(call.answers.say("managers-only", &[], true));
    }
    Planned::later(async move {
        let done = match (delete, file) {
            (Some(file), _) => deleting(&call, group, &file).await,
            (None, Some(file)) => download(&call, group, &file, wait).await,
            (None, None) => listing(&call, group, shown).await,
        };
        done.unwrap_or_else(|undone| said(&call, undone))
    })
}

/// 列一层。
async fn listing(call: &Call, group: Id, shown: usize) -> Result<Value, Undone> {
    let Some(link) = &call.link else {
        return Err(("unreachable", None));
    };
    let folder = call.args["folder"]
        .as_str()
        .filter(|folder| !folder.is_empty());
    let files = link.group_files(&group, folder).await.map_err(undone)?;
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

/// 删一个文件（施工 I-204）：QQ 说不行的照它说的答。
async fn deleting(call: &Call, group: Id, file: &str) -> Result<Value, Undone> {
    let Some(link) = &call.link else {
        return Err(("unreachable", None));
    };
    link.delete_group_file(&group, file).await.map_err(undone)?;
    tracing::info!(target: miyu_im::TARGET, venue = %call.venue, file, "group file deleted");
    Ok(call.answers.say("group-file-deleted", &[], false))
}

/// 下一个文件存进工作区。
async fn download(
    call: &Call,
    group: Id,
    file: &str,
    wait: std::time::Duration,
) -> Result<Value, Undone> {
    let Some(link) = &call.link else {
        return Err(("unreachable", None));
    };
    let url = link.group_file_url(&group, file).await.map_err(undone)?;
    // 名字照列表里的（她多半刚列过这一层；文件夹照她给的）：列不出来的照下载地址。
    let folder = call.args["folder"]
        .as_str()
        .filter(|folder| !folder.is_empty());
    let listed = link
        .group_files(&group, folder)
        .await
        .ok()
        .and_then(|files| {
            files
                .files
                .into_iter()
                .find(|one| one.id == file)
                .map(|one| one.name)
        });
    let name = saved_name(listed.as_deref(), &url, file);
    let path = fetched(
        &url,
        &name,
        (&call.cwd, std::env::home_dir()),
        (wait, UNLIMITED),
    )
    .await?;
    Ok(call.answers.say("saved", &[("path", &path)], false))
}

#[cfg(test)]
mod tests;
