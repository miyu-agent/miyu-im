//! 页面的文件（施工 I-5 上「怎么走」第 4 条）：包文件夹的 `site/`。`/` 是 `index.html`，`/<路径>` 是同一个路径的文件（施工 I-6 上：
//! 外壳、一页一个文件夹，设计 01 第八节第 8 条）；路径一段一段用 `/` 隔开，最多 [`DEPTH`] 段，每一段只收字母、数字、`.`、`-`、
//! `_`，不空、不以 `.` 开头（不出这个目录，也不给藏起来的文件）。

use std::path::Path;

/// 路径最多几段。
const DEPTH: usize = 4;

/// 请求的路径 `path` 对应的文件（相对 `site/`）；不收的是空的。
pub(super) fn name(path: &str) -> Option<&str> {
    let name = path.strip_prefix('/')?;
    if name.is_empty() {
        return Some("index.html");
    }
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_');
    let segment =
        |part: &str| !part.is_empty() && !part.starts_with('.') && part.chars().all(allowed);
    let parts: Vec<&str> = name.split('/').collect();
    (parts.len() <= DEPTH && parts.iter().all(|part| segment(part))).then_some(name)
}

/// 文件名 `name` 照扩展名给的 `Content-Type`；认不出的当字节。
pub(super) fn kind(name: &str) -> &'static str {
    match name.rsplit_once('.').map(|(_, extension)| extension) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

/// 读 `dir` 里的 `name`；没有的、是目录的、读不了的是空的。
pub(super) async fn read(dir: &Path, name: &str) -> Option<Vec<u8>> {
    tokio::fs::read(dir.join(name)).await.ok()
}
