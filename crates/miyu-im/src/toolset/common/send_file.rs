//! 传文件（施工 I-130，2026-11-04 项目主人定；施工 I-185 下下从 OneBot 驱动挪来，`qq_send_file`、`tg_send_file` 同用）：
//! `<平台>_send_file {path}`，私聊、群都给。往平台里传文件是平台的功能，不走核心的 `show`（那是给人看的，有 20 MiB 的上限）：
//! 钉好的路径交给连接（`Effect::Upload`）。QQ 交 NapCat 的 `upload_group_file`、`upload_private_file`，它自己读、自己传，im 不读进
//! 内存、不设大小上限；Telegram 由驱动读了照 `sendDocument` 传（最大 50 MB）。收不了的照平台回的原话告诉她。
//!
//! - 只认她这个会话工作区（`cwd`）里的普通文件：相对的照工作区算；照真实位置比，链接解开了再判，指出工作区的不认（场所会话是
//!   关起来的，她本来就只碰得到自己的工作区；core1 11-04）。不认的答 `no-file`。
//! - 判过的文件钉一份进 im 自己的 `outbox/`（驱动的状态目录下，名字 im 起）交出去：硬链接，链不了的（跨文件系统、Windows）拷
//!   一份。传完、没传成都删钉的那一份（im 那一头，`shown.rs`），原文件不碰。原来（施工 I-130）链好了还照 inode 再核一遍、链不了的
//!   答 `unsendable`、不是 Unix 的系统一律不传；项目主人 11-10 定「文件应该让发，别管什么身份」（施工 I-185 下下），不再核。
//! - 外人的会话（没放开的）钉文件不跟链接（项目主人 11-10 定「别跟过去，是快捷方式就照快捷方式发」，core1 审出来：判完到钉之间
//!   换成指向 `~/.ssh` 的链接就拷走了私钥）：最后一段照不跟链接打开，从同一个句柄判、拷；是链接的答 `link`，不发（交不了链接本身）。
//!   放开的会话照旧什么都能发（跟链接）。
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
    let pinned = if call.open {
        pin_open(Path::new(&call.cwd), Path::new(asked), &outbox)
    } else {
        pin(Path::new(&call.cwd), Path::new(asked), &outbox)
    };
    match pinned {
        Ok(Pinned { path, name, bytes }) => Planned::Host(Effect::Upload {
            path: plain(&path),
            name,
            size: size(bytes),
        }),
        Err(Refused::NoFile) => Planned::Now(call.answers.say("no-file", &[("path", asked)], true)),
        Err(Refused::Unsendable) => Planned::Now(call.answers.say("unsendable", &[], true)),
        Err(Refused::Link) => Planned::Now(call.answers.say("link", &[("path", asked)], true)),
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
    /// 不是她工作区里的普通文件（不在、是目录、指出去了、判的时候被换了）。
    NoFile,
    /// 钉不住：链不了也拷不了（`outbox/` 建不了、写不进）。
    Unsendable,
    /// 最后一段是链接（快捷方式）：外人的会话不跟过去，也交不了链接本身，不发（施工 I-185 下下，项目主人 11-10）。
    Link,
}

/// 外人的会话（没放开的）：判 `asked`，判过的钉进 `outbox`（模块的说明）。从工作区的真实位置一级一级往下开，每一级都不跟链接
/// （[`opened`]），从打开的句柄判是普通文件、拷（[`taken`]）；是链接的不发（项目主人 11-10：别跟过去，交不了链接本身就不发，
/// 给 AI 报错）。碰磁盘。
///
/// # Errors
///
/// 没有这个文件、不是工作区里的普通文件（带 `..`、跑出工作区的也是）的是 [`Refused::NoFile`]；路上哪一段是链接的是
/// [`Refused::Link`]；拷不进 `outbox` 的是 [`Refused::Unsendable`]。
pub(crate) fn pin(workspace: &Path, asked: &Path, outbox: &Path) -> Result<Pinned, Refused> {
    if workspace.as_os_str().is_empty() {
        return Err(Refused::NoFile);
    }
    let base = std::fs::canonicalize(workspace).map_err(|_| Refused::NoFile)?;
    let rest = below(workspace, &base, asked).ok_or(Refused::NoFile)?;
    taken(&base, &rest, outbox)
}

/// `asked` 在工作区里的那几段（纯逻辑）：相对的照工作区算，绝对的去掉工作区（照写的、照真实位置的都认）。带 `..`、跑出工作区的、
/// 一段都没有的是空的。
pub(crate) fn below(
    workspace: &Path,
    base: &Path,
    asked: &Path,
) -> Option<Vec<std::ffi::OsString>> {
    use std::path::Component;
    let relative = if asked.is_absolute() {
        asked
            .strip_prefix(base)
            .or_else(|_| asked.strip_prefix(workspace))
            .ok()?
    } else {
        asked
    };
    let mut parts = Vec::new();
    for part in relative.components() {
        match part {
            Component::Normal(name) => parts.push(name.to_os_string()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!parts.is_empty()).then_some(parts)
}

/// 工作区的真实位置 `base` 下 `rest` 那几段：打开（[`opened`]）、从打开的句柄判是普通文件，再从这个句柄拷一份进 `outbox`：判的那一份
/// 和拷的那一份是同一个，判完以后再把哪一段换成链接也拷不到外面去（core1 11-10 审出来的时间差）。
fn taken(base: &Path, rest: &[std::ffi::OsString], outbox: &Path) -> Result<Pinned, Refused> {
    let mut file = opened(base, rest)?;
    let meta = file.metadata().map_err(|_| Refused::NoFile)?;
    if meta.file_type().is_symlink() {
        return Err(Refused::Link);
    }
    if !meta.is_file() {
        return Err(Refused::NoFile);
    }
    std::fs::create_dir_all(outbox).map_err(|_| Refused::Unsendable)?;
    let path = outbox.join(fresh_name());
    let copied = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|mut out| std::io::copy(&mut file, &mut out));
    if copied.is_err() {
        if std::fs::remove_file(&path).is_err() {
            tracing::debug!(target: TARGET, path = %path.display(), "pinned file not removed");
        }
        return Err(Refused::Unsendable);
    }
    let name = rest
        .last()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Pinned {
        path,
        name,
        bytes: meta.len(),
    })
}

/// Unix：从 `base` 一级一级往下开 `rest`，每一级都照相对已经打开的那一级目录开（`openat`）、不跟链接（`O_NOFOLLOW`），目录带
/// `O_DIRECTORY`；哪一段是链接的答 [`Refused::Link`]，别的打不开的答 [`Refused::NoFile`]。
#[cfg(unix)]
fn opened(base: &Path, rest: &[std::ffi::OsString]) -> Result<std::fs::File, Refused> {
    use rustix::fs::{AtFlags, CWD, FileType, Mode, OFlags, openat, statat};
    let folder = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let refused = |dir: &dyn std::os::fd::AsFd, name: &std::ffi::OsStr| {
        let link = statat(dir, name, AtFlags::SYMLINK_NOFOLLOW)
            .is_ok_and(|stat| FileType::from_raw_mode(stat.st_mode) == FileType::Symlink);
        if link { Refused::Link } else { Refused::NoFile }
    };
    let (last, middle) = rest.split_last().ok_or(Refused::NoFile)?;
    let mut dir = openat(CWD, base, folder, Mode::empty()).map_err(|_| Refused::NoFile)?;
    for name in middle {
        dir = openat(&dir, name.as_os_str(), folder, Mode::empty())
            .map_err(|_| refused(&dir, name))?;
    }
    let file = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let fd =
        openat(&dir, last.as_os_str(), file, Mode::empty()).map_err(|_| refused(&dir, last))?;
    Ok(std::fs::File::from(fd))
}

/// 别的系统：上级目录照真实位置比在不在工作区里，最后一段打开链接本身（Windows 的 `FILE_FLAG_OPEN_REPARSE_POINT`）。中间几段不
/// 一级一级开：外人会话的沙盒在 Windows 上暂缓（设计 62），AI 本来就能用命令拷文件，这里不另加。
#[cfg(not(unix))]
fn opened(base: &Path, rest: &[std::ffi::OsString]) -> Result<std::fs::File, Refused> {
    let at: PathBuf = rest
        .iter()
        .fold(base.to_path_buf(), |at, name| at.join(name));
    let parent = at.parent().ok_or(Refused::NoFile)?;
    let dir = std::fs::canonicalize(parent).map_err(|_| Refused::NoFile)?;
    if !dir.starts_with(base) {
        return Err(Refused::NoFile);
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        /// `FILE_FLAG_OPEN_REPARSE_POINT`：打开链接本身，不跟过去。
        const OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(OPEN_REPARSE_POINT);
    }
    match options.open(&at) {
        Ok(file) => Ok(file),
        Err(_)
            if std::fs::symlink_metadata(&at).is_ok_and(|meta| meta.file_type().is_symlink()) =>
        {
            Err(Refused::Link)
        }
        Err(_) => Err(Refused::NoFile),
    }
}

/// 把真实位置 `real`（`bytes` 大）钉一份进 `outbox`：硬链接，链不了的拷一份。
fn pinned(real: &Path, bytes: u64, outbox: &Path) -> Result<Pinned, Refused> {
    std::fs::create_dir_all(outbox).map_err(|_| Refused::Unsendable)?;
    let path = outbox.join(fresh_name());
    if std::fs::hard_link(real, &path).is_err() {
        std::fs::copy(real, &path).map_err(|_| Refused::Unsendable)?;
    }
    let name = real
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Pinned { path, name, bytes })
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
    pinned(&real, meta.len(), outbox)
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
