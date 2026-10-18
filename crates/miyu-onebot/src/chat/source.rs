//! 带的东西在哪（施工 I-2 上从 `onebot/media.rs` 挪来）：驱动读回应读出来，宿主照它去取。

use std::path::PathBuf;

/// 东西在哪：照试的先后（2026-10-11 主会话定）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// 下载地址：只认 `http`、`https`。
    Url(String),
    /// 本机的路径：NapCat 和桥在同一台机器上才读得到（在 Docker 里的读不到）。
    Path(PathBuf),
    /// 内容的 base64（去掉了 `base64://`）。
    Base64(String),
}
