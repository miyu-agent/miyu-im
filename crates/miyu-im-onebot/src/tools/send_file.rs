//! 传文件（施工 I-130，2026-11-04 项目主人定）：`qq_send_file {path}`，私聊、群都给。往 QQ 里传文件是平台的功能，不走核心的
//! `show`（那是给人看的，有 20 MiB 的上限）：路径原样交给 NapCat 的 `upload_group_file`、`upload_private_file`，它自己读、自己传，
//! im 不读进内存，不设大小上限，QQ 收不了的照 NapCat 回的原话告诉她。
//!
//! - 只认她这个会话工作区（`cwd`）里的普通文件：相对的照工作区算；照真实位置比，链接解开了再判，指出工作区的不认（场所会话是
//!   关起来的，她本来就只碰得到自己的工作区；core1 11-04）。不认的答 `no-file`。
//! - 对端读不到本机的文件（NapCat 不是从本机连进来的）答 `not-local`；没连着答 `unreachable`。
//! - 认了的交给 im（`Effect::Upload`）：另起任务传，当场答 `uploading`；没传成的照 `show` 的文件记 `unshown`。

use std::path::{Path, PathBuf};

use miyu_im::toolset::{Call, Effect, Planned};

use super::args::bad_args;
use super::group_files::size;

/// `qq_send_file` 的一次调用（模块的说明）。
pub(super) fn send_file(call: &Call) -> Planned {
    let Some(asked) = call.args["path"]
        .as_str()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return bad_args(call, "path is required");
    };
    let Some(link) = &call.link else {
        return Planned::Now(call.answers.say("unreachable", &[], true));
    };
    if !link.local_files() {
        return Planned::Now(call.answers.say("not-local", &[], true));
    }
    match within(Path::new(&call.cwd), Path::new(asked)) {
        Some((path, bytes)) => Planned::Host(Effect::Upload {
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: plain(&path),
            size: size(bytes),
        }),
        None => Planned::Now(call.answers.say("no-file", &[("path", asked)], true)),
    }
}

/// `asked`（相对的照工作区 `workspace` 算）的真实位置在工作区的真实位置里、是普通文件：交回真实位置和大小。工作区没给的、
/// 不在的、解不开的、指出去的、是目录的都是空的。碰磁盘。
pub(crate) fn within(workspace: &Path, asked: &Path) -> Option<(PathBuf, u64)> {
    if workspace.as_os_str().is_empty() {
        return None;
    }
    let base = std::fs::canonicalize(workspace).ok()?;
    let real = std::fs::canonicalize(workspace.join(asked)).ok()?;
    if !real.starts_with(&base) {
        return None;
    }
    let meta = std::fs::metadata(&real).ok()?;
    meta.is_file().then_some((real, meta.len()))
}

/// 交给 NapCat 的写法：Windows 上 `canonicalize` 给的 `\\?\` 前缀去掉（NapCat 照普通路径读）。
fn plain(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
}

#[cfg(test)]
mod tests;
