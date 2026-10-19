//! `miyu im web`（施工 I-6 上，设计 01 第九节第 3、4 条）：带交接票打开 im 自己的页面（施工 O-28 补起原来是转给网页软件的
//! `miyu web --package im`，打开它框里的后台页；im 有了自己的页面以后换成这个）。
//!
//! 1. 照终端的样子连已经在跑的核心（本机令牌，`open::Core::connect_running`；核心没在跑的 im 也没在跑），问 `extension.status`：
//!    im 在跑、状态文件的进程号对得上、写着页面服务的端口（`web`），才往下；不是的说一句「先 `miyu im start`」，退出码 1。
//! 2. 要一张交接票（`login.ticket`，本机令牌连上的是管理员），拼网址：`http://127.0.0.1:<端口>/#miyu=<base64url(JSON)>`，JSON 是
//!    `{"v": 1, "ticket", "lang"}`（设计第九节第 4 条的写法；主题照页面自己的，跟随系统）。片段不发给服务器、不进访问日志。
//! 3. 交给系统打开（`open::Browser`），在标准输出上印一行网址：SSH 进来的、打不开浏览器的能复制（票 60 秒、一次性）。
//!
//! 连不上核心、核心拒了：照原话说，退出码 1。

use std::io::Write;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};

use miyu_client::open::{Browser, Core};
use miyu_client::places::DataRoot;

use crate::status_file;
use crate::texts::Texts;
use crate::{PACKAGE, PROGRAM};

/// 没办成的退出码。
const FAILED: u8 = 1;

/// 打开 im 的页面：数据根 `root`，照 `texts` 说话，网址交给 `browser`；网址印在 `out`，没办成的原因印在 `err`。交回退出码。
pub async fn web(
    root: &DataRoot,
    texts: &Texts,
    browser: &dyn Browser,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let mut core = match Core::connect_running(root, PROGRAM).await {
        Ok(core) => core,
        Err(refused) if refused.reason == "unreachable" => {
            say(err, &texts.web_not_running());
            return FAILED;
        }
        Err(refused) => {
            say(err, &texts.web_failed(&refused.to_string()));
            return FAILED;
        }
    };
    let status = match core.call(PACKAGE, "extension.status", json!({})).await {
        Ok(status) => status,
        Err(refused) => {
            say(err, &texts.web_failed(&refused.to_string()));
            return FAILED;
        }
    };
    let Some(port) = page_port(&status, status_file::read(root).as_ref()) else {
        say(err, &texts.web_not_running());
        return FAILED;
    };
    let ticket = match core.call(PACKAGE, "login.ticket", json!({})).await {
        Ok(ticket) => ticket["ticket"].as_str().unwrap_or_default().to_string(),
        Err(refused) => {
            say(err, &texts.web_failed(&refused.to_string()));
            return FAILED;
        }
    };
    let url = url(port, &ticket, texts.language());
    let opened = browser.open(&url);
    say(out, &texts.web_opened(&url, opened));
    0
}

/// im 在跑（`extension.status` 里它那一项是 `running`）、状态文件的进程号对得上的，状态文件写着的页面服务的端口。
fn page_port(status: &Value, board: Option<&Value>) -> Option<u16> {
    let entry = status["extensions"]
        .as_array()?
        .iter()
        .find(|one| one["package"] == PACKAGE)?;
    let board = board?;
    let running =
        entry["state"] == "running" && entry["pid"].is_u64() && board["pid"] == entry["pid"];
    running
        .then(|| board["web"].as_u64())
        .flatten()
        .and_then(|port| u16::try_from(port).ok())
}

/// 页面的网址：端口 `port`，片段带交接票 `ticket` 和语言 `lang`。
pub fn url(port: u16, ticket: &str, lang: &str) -> String {
    let handed = json!({"v": 1, "ticket": ticket, "lang": lang});
    let encoded = URL_SAFE_NO_PAD.encode(handed.to_string());
    format!("http://127.0.0.1:{port}/#miyu={encoded}")
}

/// 印一行；印不出来也没有别处可说了。
fn say(to: &mut dyn Write, line: &str) {
    if writeln!(to, "{line}").is_err() {
        // 标准输出、标准错误关了：没有别处可说。
    }
}
