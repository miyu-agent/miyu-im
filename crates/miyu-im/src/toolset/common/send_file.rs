//! 传文件（施工 I-130，2026-11-04 项目主人定；施工 I-185 下下从 OneBot 驱动挪来，`qq_send_file`、`tg_send_file` 同用）：
//! `<平台>_send_file {path}`，私聊、群都给。往平台里传文件是平台的功能，不走核心的 `show`（那是给人看的，有 20 MiB 的上限）：
//! 钉好的路径交给连接（`Effect::Upload`）。QQ 交 NapCat 的 `upload_group_file`、`upload_private_file`，它自己读、自己传，im 不读进
//! 内存、不设大小上限；Telegram 由驱动读了照 `sendDocument` 传（最大 50 MB）。收不了的照平台回的原话告诉她。
//!
//! - 只认她这个会话工作区（`cwd`）里的普通文件：相对的照工作区算；照真实位置比，链接解开了再判，指出工作区的不认（场所会话是
//!   关起来的，她本来就只碰得到自己的工作区；core1 11-04）。没有这个文件、不是普通文件的答 `no-file`，在工作区外面的答 `outside`
//!   （施工 I-190：原来一律答「不是工作区里的文件」，她照它当成了白名单）。
//! - 工作区、她写的路径开头的 `~` 换成家目录（施工 I-190，`media::expand`）：从旧版导进来的会话工作区记的是 `~`，核心原样交过来，
//!   她写路径也爱写 `~/…`；原来都没换，照工作区里叫 `~` 的目录找，什么都找不到。
//! - 判完到 NapCat 真去读之间有时间差（core1 11-04 审出来的）：她能把那个文件换成指出去的链接，NapCat 不在沙盒里、以主人的身份
//!   读。所以判过的文件先硬链接进 im 自己的 `outbox/`（驱动的状态目录下，名字 im 起），钉住那个 inode，交给 NapCat 的是这一份；
//!   链接好了再核一遍：原路径照样解在工作区里、是同一个 inode，和钉住的也是同一个，不然删掉、不认。传完、没传成都删（im 那一头，
//!   `shown.rs`）。硬链接不了的（跨文件系统；不是 Unix 的系统上核不了 inode）答 `unsendable`，不退回拷一份：拷也有同样的时间差。
//! - 对端读不到本机的文件（NapCat 不是从本机连进来的）答 `not-local`；没连着答 `unreachable`。
//! - 认了的交给 im（`Effect::Upload`）：另起任务传，当场答 `uploading`；没传成的照 `show` 的文件记 `unshown`。
//! - 放开的会话（施工 LV-4，`Call::open`：主人本人叫的、完全放开）发什么文件都行（项目主人 11-10 定）：她本来就读得到，不比工作区、
//!   不再核时间差，照旧钉一份进 `outbox/`（im 传完删的是钉的那一份，不能交原文件）；硬链接不了的拷一份。

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::TARGET;
use crate::toolset::{Call, Effect, Planned};

use super::args::bad_args;

/// 钉住的文件放在驱动状态目录下的哪里。
const OUTBOX: &str = "outbox";

/// `qq_send_file` 的一次调用（模块的说明）：`upload` 是传文件最多等多久，`outbox/` 里比它还老的是没删掉的，顺手清。
pub fn send_file(call: &Call, upload: Duration) -> Planned {
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
    let outbox = call.store.join(OUTBOX);
    sweep(&outbox, upload);
    let (workspace, wanted) = paths(&call.cwd, asked, std::env::home_dir().as_deref());
    let pinned = if call.open {
        pin_open(&workspace, &wanted, &outbox)
    } else {
        pin(&workspace, &wanted, &outbox)
    };
    match pinned {
        Ok(Pinned { path, name, bytes }) => Planned::Host(Effect::Upload {
            path: plain(&path),
            name,
            size: size(bytes),
        }),
        Err(Refused::NoFile) => Planned::Now(call.answers.say("no-file", &[("path", asked)], true)),
        Err(Refused::Outside) => {
            Planned::Now(call.answers.say("outside", &[("path", asked)], true))
        }
        Err(Refused::Unsendable) => Planned::Now(call.answers.say("unsendable", &[], true)),
    }
}

/// 钉住了的一份。不是 Unix 的系统上不传，造不出来。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pinned {
    /// `outbox/` 里的那一份：交给 NapCat 的路径。
    pub(crate) path: PathBuf,
    /// 原来的文件名：传上去叫这个。
    pub(crate) name: String,
    /// 多大。
    pub(crate) bytes: u64,
}

/// 为什么不传。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refused {
    /// 没有这个文件、不是普通文件、判的时候被换了。
    NoFile,
    /// 在工作区外面（指出去的链接也算）：只给没放开的会话（施工 I-190 从 [`Refused::NoFile`] 分出来）。
    Outside,
    /// 钉不住：硬链接不了（跨文件系统），或者这个系统上核不了 inode。
    Unsendable,
}

/// `asked`（相对的照工作区 `workspace` 算）的真实位置在工作区的真实位置里、是普通文件：交回真实位置和它的元数据，不是的交回
/// 为什么。碰磁盘。
fn checked(workspace: &Path, asked: &Path) -> Result<(PathBuf, std::fs::Metadata), Refused> {
    if workspace.as_os_str().is_empty() {
        return Err(Refused::NoFile);
    }
    let base = std::fs::canonicalize(workspace).map_err(|_| Refused::NoFile)?;
    let real = std::fs::canonicalize(workspace.join(asked)).map_err(|_| Refused::NoFile)?;
    if !real.starts_with(&base) {
        return Err(Refused::Outside);
    }
    let meta = std::fs::symlink_metadata(&real).map_err(|_| Refused::NoFile)?;
    if meta.is_file() {
        Ok((real, meta))
    } else {
        Err(Refused::NoFile)
    }
}

/// 工作区 `cwd`、她写的路径 `asked`，开头的 `~` 换成家目录 `home`（纯逻辑，施工 I-190）；家目录读不出的照原样。
pub(crate) fn paths(cwd: &str, asked: &str, home: Option<&Path>) -> (PathBuf, PathBuf) {
    let expand =
        |text: &str| crate::media::expand(text, home).unwrap_or_else(|_| PathBuf::from(text));
    (expand(cwd), expand(asked))
}

/// 判 `asked`，判过的硬链接进 `outbox`（模块的说明），链接好再核一遍。碰磁盘。
///
/// # Errors
///
/// 不是工作区里的普通文件、核的时候对不上的是 [`Refused::NoFile`]；钉不住的是 [`Refused::Unsendable`]。
#[cfg(unix)]
pub(crate) fn pin(workspace: &Path, asked: &Path, outbox: &Path) -> Result<Pinned, Refused> {
    use std::os::unix::fs::MetadataExt;
    let same = |one: &std::fs::Metadata, other: &std::fs::Metadata| {
        one.dev() == other.dev() && one.ino() == other.ino()
    };
    let (real, meta) = checked(workspace, asked)?;
    std::fs::create_dir_all(outbox).map_err(|_| Refused::Unsendable)?;
    let path = outbox.join(fresh_name());
    std::fs::hard_link(&real, &path).map_err(|_| Refused::Unsendable)?;
    let held = std::fs::symlink_metadata(&path).ok();
    let again = checked(workspace, asked).ok();
    let kept = match (&held, &again) {
        (Some(held), Some((real_again, meta_again))) => {
            held.is_file() && *real_again == real && same(held, &meta) && same(meta_again, &meta)
        }
        _ => false,
    };
    if !kept {
        if std::fs::remove_file(&path).is_err() {
            tracing::debug!(target: TARGET, path = %path.display(), "pinned file not removed");
        }
        return Err(Refused::NoFile);
    }
    let name = real
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Pinned {
        path,
        name,
        bytes: meta.len(),
    })
}

/// 不是 Unix 的系统上核不了 inode：判过照样不传（模块的说明）。
///
/// # Errors
///
/// 不是工作区里的普通文件的是 [`Refused::NoFile`]，别的一律 [`Refused::Unsendable`]。
#[cfg(not(unix))]
pub(crate) fn pin(workspace: &Path, asked: &Path, _outbox: &Path) -> Result<Pinned, Refused> {
    match checked(workspace, asked) {
        Err(refused) => Err(refused),
        Ok(_) => Err(Refused::Unsendable),
    }
}

/// 放开的会话（施工 LV-4）：`asked`（相对的照工作区 `workspace` 算，工作区是空的只认绝对路径）的真实位置是普通文件就钉一份进
/// `outbox`：硬链接，链不了的（跨文件系统、不是 Unix）拷一份。碰磁盘。
///
/// # Errors
///
/// 不在、不是普通文件的是 [`Refused::NoFile`]；链不了也拷不了的是 [`Refused::Unsendable`]。
pub(crate) fn pin_open(workspace: &Path, asked: &Path, outbox: &Path) -> Result<Pinned, Refused> {
    if workspace.as_os_str().is_empty() && asked.is_relative() {
        return Err(Refused::NoFile);
    }
    let real = std::fs::canonicalize(workspace.join(asked)).map_err(|_| Refused::NoFile)?;
    let meta = std::fs::symlink_metadata(&real).map_err(|_| Refused::NoFile)?;
    if !meta.is_file() {
        return Err(Refused::NoFile);
    }
    std::fs::create_dir_all(outbox).map_err(|_| Refused::Unsendable)?;
    let path = outbox.join(fresh_name());
    if std::fs::hard_link(&real, &path).is_err() {
        std::fs::copy(&real, &path).map_err(|_| Refused::Unsendable)?;
    }
    let name = real
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Pinned {
        path,
        name,
        bytes: meta.len(),
    })
}

/// `outbox/` 里一份的名字：钉住的那一刻（秒）、进程号、进程里第几份。名字带时刻，清的时候照它认老的（硬链接的修改时刻是原文件的）。
fn fresh_name() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}-{}",
        now_seconds(),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

/// 此刻：秒的 Unix 时间。
fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// 清掉 `outbox` 里钉住超过 `upload` 再加一分钟的：传的任务早就不等了，是桥崩了、重启前没删掉的。删不了的不管。
fn sweep(outbox: &Path, upload: Duration) {
    let Ok(entries) = std::fs::read_dir(outbox) else {
        return;
    };
    let cutoff = now_seconds().saturating_sub(upload.as_secs() + 60);
    for entry in entries.flatten() {
        let name = entry.file_name();
        let pinned_at = name
            .to_str()
            .and_then(|name| name.split('-').next())
            .and_then(|seconds| seconds.parse::<u64>().ok());
        if pinned_at.is_some_and(|at| at < cutoff) && std::fs::remove_file(entry.path()).is_err() {
            tracing::debug!(target: TARGET, path = %entry.path().display(), "stale pinned file not removed");
        }
    }
}

/// 交给 NapCat 的写法：Windows 上 `canonicalize` 给的 `\\?\` 前缀去掉（NapCat 照普通路径读）。
fn plain(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
}

/// 一个大小，照人的单位：`512 B`、`1.2 KB`、`3.4 MB`、`1.0 GB`（QQ 的群文件列表也照它写）。
#[must_use]
pub fn size(bytes: u64) -> String {
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

#[cfg(test)]
mod tests;
