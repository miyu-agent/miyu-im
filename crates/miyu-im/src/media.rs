//! 取到手、存下来（施工 O-33，`onebot.md` 第一条「平台工具（二）」第 4、5 条）：NapCat 说东西在哪（`onebot::media::sources`），
//! 这里照先后拿到字节；视频、文件写进会话工作区的 `qq-files/`。
//!
//! - 拿字节（[`bytes`]）：先下 `http`、`https` 的地址（核心的 HTTP 客户端 `miyu-http`：连接的时限照它，整个最多 `fetch_seconds`；
//!   回环地址不走代理，别的照环境变量），再读本机路径（NapCat 和桥在同一台机器上才有），再解 base64（NapCat 开了
//!   `enableLocalFile2Url` 才给）；都不行的交回每一样为什么（2026-10-11 主会话定：很多人的 NapCat 跑在 Docker 里，路径读不到）。
//!   图有上限（2026-10-22 项目主人定，漏洞排查报的：陌生人 @ 她就会自动取图）：`limit` 字节（`bridge.json` 的 `image_max_mb`，
//!   出厂 20 MB），下载读到上限就停、本机路径先看大小、base64 照长度估，超了不要；视频、文件照旧不设上限（2026-10-11 项目主人
//!   定）：整个读进内存，大的视频吃内存（施工单「风险」）。
//! - 存（[`save`]）：路径从核心给的 `cwd` 来，`~` 自己换成家目录；只写进它下面的 `qq-files/`，文件名去掉路径分隔符、开头的点；
//!   先写临时文件再改名（同一个文件取两次照新的换掉，读的一方看不到写了一半的）；`qq-files` 和文件落到的真实位置都要还在
//!   工作区里，不跟链接走（`qq-files` 是链接的不写，文件名那里本来是链接的被换掉、不写进它指的地方）。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use miyu_client::http::{Get, Got, Proxy};

use crate::chat::{CallError, Fetch, Fetched, Link, Source, To};

mod numbered;
pub use numbered::{Numbering, find as find_in_forwards};

/// 没取到。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Missed {
    /// NapCat 回了失败：它说的（去掉首尾空白、截到 200 个字符，同出站队列的 `rejected`）。
    Failed(String),
    /// 等不到 NapCat 回。
    Unanswered,
    /// 那个机器人号没连着、连接断了。
    Unreachable,
    /// NapCat 回了，东西拿不到手（[`bytes`] 说的为什么）。
    Unfetched(String),
}

impl Missed {
    /// 记运行日志、答她的那一句的名字（`tool-results/<名字>.txt`）。
    pub fn name(&self) -> &'static str {
        match self {
            Missed::Failed(_) => "failed",
            Missed::Unanswered => "unanswered",
            Missed::Unreachable => "unreachable",
            Missed::Unfetched(_) => "unfetched",
        }
    }
}

/// NapCat 那一次调用没成算成哪一种。
fn missed(error: CallError) -> Missed {
    match error {
        CallError::Failed { said, .. } => Missed::Failed(said),
        CallError::Timeout => Missed::Unanswered,
        CallError::Closed => Missed::Unreachable,
    }
}

/// 取回来的那一条在不在会话 `to` 里（纯逻辑，施工 O-35 核对归属；群里的 O-33 起就查；施工 I-39 从驱动挪来，`chat.media` 也照它）：
/// 群里是这个群的，私聊里是这个人在私聊里发的。编号是她（或别的包）给的，人打得出假的 `[msg=…]`：这个人在群里说的、别人的私聊、
/// 她自己发的都当没有。
pub fn owned(fetched: &Fetched, to: To) -> bool {
    match to {
        To::Group(group) => fetched.group == Some(group),
        To::Private(user) => fetched.private && fetched.user == Some(user),
    }
}

/// 经连接 `link` 取平台编号是 `msg` 的那一条消息（[`Link::message`]）；等多久照一次调用。
///
/// # Errors
///
/// 平台回了失败（没有这一条、过期了）、等不到、没连着。
pub async fn message(link: &dyn Link, msg: &str) -> Result<Fetched, Missed> {
    link.message(msg).await.map_err(missed)
}

/// 经连接 `link` 照 `fetch` 取编号是 `id` 的那一样（[`Link::fetch`]），再照平台说的地方拿到字节（[`bytes`]，最多 `limit` 字节）。
/// 平台回应、下载各最多等 `wait`。交回字节和平台说的文件名。
///
/// # Errors
///
/// 平台回了失败、等不到、没连着；回了但哪儿都拿不到。
pub async fn take(
    link: &dyn Link,
    fetch: Fetch,
    id: &str,
    (wait, limit): (Duration, usize),
) -> Result<(Vec<u8>, Option<String>), Missed> {
    let located = link.fetch(fetch, id, wait).await.map_err(missed)?;
    let bytes = bytes(&located.sources, wait, limit)
        .await
        .map_err(Missed::Unfetched)?;
    Ok((bytes, located.name))
}

/// 工作区里放取来的视频、文件的目录（施工单「要定的」第 3 条）。
pub const QQ_FILES: &str = "qq-files";

/// 文件名里名字那一段最多几个字符：前面还有消息编号和第几个，整个不超过文件系统的 255 字节。
const NAME: usize = 80;

/// 不设上限（视频、文件，2026-10-11 项目主人定）。
pub const UNLIMITED: usize = usize::MAX;

/// 照 `sources` 的先后拿字节，下载整个最多 `timeout`，最多 `limit` 字节（超了的不要，同「拿不到」）。都不行的交回每一样为什么，
/// 英文，`; ` 隔开；一样都没有的说没有。
///
/// # Errors
///
/// 地址下不下来、路径读不了、base64 解不出、超了上限：都不行的时候。
pub async fn bytes(sources: &[Source], timeout: Duration, limit: usize) -> Result<Vec<u8>, String> {
    let mut why = Vec::new();
    for source in sources {
        let got = match source {
            Source::Url(url) => download(url, timeout, limit).await,
            Source::Path(path) => read(path.clone(), limit).await,
            // 解出来的约是字符数的四分之三：照它先挡，免得大块字先解进内存。
            Source::Base64(encoded) if encoded.len() / 4 * 3 > limit => {
                Err(format!("base64: {}", over(limit)))
            }
            Source::Base64(encoded) => STANDARD
                .decode(encoded)
                .map_err(|_| "base64: not valid".to_string()),
        };
        match got {
            Ok(bytes) => return Ok(bytes),
            Err(error) => why.push(error),
        }
    }
    if why.is_empty() {
        return Err("no url, path or base64 in the answer".to_string());
    }
    Err(why.join("; "))
}

/// 段里写了大小（`file_size`）、大过 `limit` 字节的：先挡，不下（施工 I-23）。没写的照下载时读到的算。
pub fn too_large(item: &crate::chat::Media, limit: usize) -> bool {
    item.size
        .is_some_and(|size| usize::try_from(size).map_or(true, |size| size > limit))
}

/// 超了上限 `limit` 字节的那一句（英文，同别的原因），按 MB 说：`over 20 MB`。
pub fn over(limit: usize) -> String {
    format!("over {} MB", limit.div_ceil(1024 * 1024))
}

/// 下一个地址：回环的不走代理（开发机上设了代理也连得上本机），别的照环境变量。读到 `limit` 字节就停（核心的 HTTP 客户端照它
/// 截）。
async fn download(url: &str, timeout: Duration, limit: usize) -> Result<Vec<u8>, String> {
    let proxy = if miyu_client::http::is_loopback_url(url) {
        Proxy::Off
    } else {
        Proxy::FromEnvironment
    };
    let client = miyu_client::http::fetcher(proxy).map_err(|error| format!("url: {error}"))?;
    let got = miyu_client::http::get(Get {
        client: &client,
        url,
        headers: &[],
        etag: None,
        timeout,
        limit,
    })
    .await
    .map_err(|error| format!("url: {error}"))?;
    match got {
        Got::Body { bytes, .. } => Ok(bytes),
        // 没带 `ETag` 问，不会回 304。
        Got::NotModified => Err("url: not modified".to_string()),
    }
}

/// 读本机的一个文件（在阻塞线程里）：要是普通文件，不大过 `limit` 字节（先看大小，不读进来）。
async fn read(path: PathBuf, limit: usize) -> Result<Vec<u8>, String> {
    tokio::task::spawn_blocking(move || {
        if !path.is_file() {
            return Err("path: not a file here".to_string());
        }
        let size = std::fs::metadata(&path)
            .map_err(|error| format!("path: {error}"))?
            .len();
        if usize::try_from(size).map_or(true, |size| size > limit) {
            return Err(format!("path: {}", over(limit)));
        }
        std::fs::read(&path).map_err(|error| format!("path: {error}"))
    })
    .await
    .map_err(|error| format!("path: {error}"))?
}

/// 存进工作区的文件名：`<消息编号>-<第几个>`，有名字的接 `-<名字>`；名字去掉路径分隔符、Windows 不许的字符和控制字符，去掉
/// 开头的点和空白、结尾的点和空白（Windows 会吃掉），留前 `NAME`（80）个字符；洗完是空的不接。
pub fn file_name(msg: &str, index: usize, name: Option<&str>) -> String {
    let cleaned: String = name
        .unwrap_or_default()
        .chars()
        .filter(|char| !char.is_control() && !"/\\:*?\"<>|".contains(*char))
        .collect();
    let cleaned: String = cleaned
        .trim_start_matches(|char: char| char == '.' || char.is_whitespace())
        .trim_end_matches(|char: char| char == '.' || char.is_whitespace())
        .chars()
        .take(NAME)
        .collect();
    let cleaned = cleaned.trim_end_matches(|char: char| char == '.' || char.is_whitespace());
    if cleaned.is_empty() {
        format!("{msg}-{index}")
    } else {
        format!("{msg}-{index}-{cleaned}")
    }
}

/// 把 `bytes` 存成工作区 `cwd`（`~` 换成家目录 `home`）下 `qq-files/` 里的 `name`（照取东西时 `file_name` 洗过的），交回落到的真实路径。
/// 在阻塞线程里调。
///
/// # Errors
///
/// 工作区不是绝对路径、没有，`qq-files` 不是目录（链接也算不是）、落到了工作区外面、写不了：英文的一句。
pub fn save(cwd: &str, home: Option<&Path>, name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let workspace = expand(cwd, home)?;
    if !workspace.is_absolute() {
        return Err(format!("workspace {cwd} is not an absolute path"));
    }
    let workspace = std::fs::canonicalize(&workspace)
        .map_err(|error| format!("workspace {}: {error}", workspace.display()))?;
    let dir = workspace.join(QQ_FILES);
    match std::fs::symlink_metadata(&dir) {
        Ok(found) if found.is_dir() => {}
        Ok(_) => return Err(format!("{QQ_FILES} is not a folder")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(&dir).map_err(|error| format!("{QQ_FILES}: {error}"))?;
        }
        Err(error) => return Err(format!("{QQ_FILES}: {error}")),
    }
    let dir = std::fs::canonicalize(&dir).map_err(|error| format!("{QQ_FILES}: {error}"))?;
    if !dir.starts_with(&workspace) {
        return Err(format!("{QQ_FILES} is outside the workspace"));
    }
    let target = dir.join(name);
    if target.parent() != Some(dir.as_path()) {
        return Err("the file name is not a plain name".to_string());
    }
    if std::fs::symlink_metadata(&target).is_ok_and(|found| found.is_dir()) {
        return Err(format!("{name} is a folder"));
    }
    let temp = dir.join(format!(".{}.part", unique()));
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .and_then(|mut file| file.write_all(bytes).and_then(|()| file.sync_all()))
        .and_then(|()| std::fs::rename(&temp, &target));
    if let Err(error) = written {
        if std::fs::remove_file(&temp).is_err() {
            // 没建起来，或者已经改了名：没有要清的。
        }
        return Err(format!("{name}: {error}"));
    }
    Ok(target)
}

/// 工作区的路径：`~`、`~/…`（Windows 上 `~\…` 也认）换成家目录；别的照原样（`~别人` 也是，之后当相对路径拒掉）。家目录读不出
/// 的说清楚。
fn expand(cwd: &str, home: Option<&Path>) -> Result<PathBuf, String> {
    let rest = match cwd.strip_prefix('~') {
        None => return Ok(PathBuf::from(cwd)),
        Some("") => "",
        Some(rest) => match rest.strip_prefix(['/', '\\']) {
            Some(rest) => rest,
            // `~别人`：不认。
            None => return Ok(PathBuf::from(cwd)),
        },
    };
    let home = home.ok_or("no home folder to expand ~")?;
    Ok(home.join(rest))
}

/// 临时文件名里的一段：16 位随机的十六进制；取不到随机数的用进程号和此刻的纳秒（同 `core.rs` 的编号前缀）。
fn unique() -> String {
    let mut bytes = [0u8; 8];
    match getrandom::fill(&mut bytes) {
        Ok(()) => bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
        Err(_) => {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos());
            format!("{:x}{nanos:x}", std::process::id())
        }
    }
}

#[cfg(test)]
mod tests;

/// 照开头几个字节认得出的图的扩展名（施工 I-16：群文件认不出的不当 JPEG）：PNG、GIF、WebP、JPEG，别的是空的。
#[must_use]
pub fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Some("jpg");
    }
    Some(extension(bytes)).filter(|found| *found != "jpg")
}

/// 照开头几个字节认图的扩展名（施工 I-6 中从 `core/route/lookup.rs` 挪来：页面服务给头像也照它）：PNG、GIF、WebP，别的当 JPEG（QQ 的头像多是 JPEG）。
pub fn extension(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG") {
        "png"
    } else if bytes.starts_with(b"GIF8") {
        "gif"
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "webp"
    } else {
        "jpg"
    }
}
