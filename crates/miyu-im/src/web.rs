//! `miyu im web`（施工 I-6 上，设计 01 第九节第 3、4 条）：带交接票打开 im 自己的页面（施工 O-28 补起原来是转给网页软件的
//! `miyu web --package im`，打开它框里的后台页；im 有了自己的页面以后换成这个）。
//!
//! 1. 照 `miyu im start` 的样子连核心（本机令牌，没在跑就拉起），问 `extension.status`：im 在跑、状态文件的进程号对得上、写着
//!    页面服务的端口（`web`），往下。没开的（施工 I-178，项目主人 11-09：原来说一句「先 `miyu im start`」就退出）照 `start`
//!    打开开关、说一句已启动；停下了的照 `restart` 再拉起；接着等它的页面服务听上（[`READY`] 以内）。停下了的照 `status` 说
//!    为什么，等不到的说没起来、用 `miyu im status` 看，退出码 1。核心的连接只有一问一答、收不到 `extension.changed`，这里
//!    隔 100 毫秒问一次，一次性的命令、有上限。
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

use std::time::Duration;

use crate::control::{CoreCommand, Line, Report, describe};
use crate::serve::Failure;
use crate::status_file;
use crate::texts::Texts;
use crate::{PACKAGE, PROGRAM};

/// 没办成的退出码。
const FAILED: u8 = 1;

/// 打开了开关以后最多等页面服务听上多久。
pub const READY: Duration = Duration::from_secs(30);

/// 等的时候隔多久问一次。
const STEP: Duration = Duration::from_millis(100);

/// 打开 im 的页面：数据根 `root`，核心没在跑照 `launch` 拉起，照 `texts` 说话，网址交给 `browser`；网址印在 `out`，没办成的原因
/// 印在 `err`。交回退出码。
pub async fn web(
    root: &DataRoot,
    launch: &CoreCommand,
    texts: &Texts,
    browser: &dyn Browser,
    (out, err): (&mut dyn Write, &mut dyn Write),
) -> u8 {
    let mut core = match Core::connect(root, launch, PROGRAM).await {
        Ok(core) => core,
        Err(refused) => {
            say(err, &texts.failure(&Failure::Core(refused.to_string())));
            return FAILED;
        }
    };
    let port = match ready(&mut core, root, texts, out).await {
        Ok(port) => port,
        Err(lines) => {
            for line in lines {
                say(err, &line);
            }
            return FAILED;
        }
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

/// im 页面服务的端口（模块的说明第 1 条）：没开的打开、停下了的再拉起（说一句在 `out`），等它听上。没办成的交回要说的几行。
async fn ready(
    core: &mut Core,
    root: &DataRoot,
    texts: &Texts,
    out: &mut dyn Write,
) -> Result<u16, Vec<String>> {
    let failed = |refused: miyu_client::open::Refused| vec![texts.web_failed(&refused.to_string())];
    let status = core
        .call(PACKAGE, "extension.status", json!({}))
        .await
        .map_err(failed)?;
    if let Some(port) = page_port(&status, status_file::read(root).as_ref()) {
        return Ok(port);
    }
    let entry = im_entry(&status).ok_or_else(|| vec![texts.report(&Report::Missing)])?;
    let started = if entry["on"] != true {
        Some(("extension.enable", Report::Started))
    } else if entry["state"] == "stopped" {
        Some(("extension.restart", Report::Restarted))
    } else {
        None
    };
    if let Some((method, done)) = started {
        core.call(PACKAGE, method, json!({"package": PACKAGE}))
            .await
            .map_err(|said| vec![said.to_string()])?;
        say(out, &texts.report(&done));
    }
    let deadline = tokio::time::Instant::now() + READY;
    loop {
        tokio::time::sleep(STEP).await;
        let status = core
            .call(PACKAGE, "extension.status", json!({}))
            .await
            .map_err(failed)?;
        let board = status_file::read(root);
        if let Some(port) = page_port(&status, board.as_ref()) {
            return Ok(port);
        }
        let entry = im_entry(&status).ok_or_else(|| vec![texts.report(&Report::Missing)])?;
        if entry["state"] == "stopped" {
            return Err(describe(&entry, board.as_ref())
                .into_iter()
                .map(|line| match line {
                    Line::Say(report) => texts.report(&report),
                    Line::Tail(tail) => format!("  {tail}"),
                })
                .collect());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(vec![texts.web_not_ready()]);
        }
    }
}

/// `extension.status` 里 im 的那一项。
fn im_entry(status: &Value) -> Option<Value> {
    status["extensions"]
        .as_array()?
        .iter()
        .find(|one| one["package"] == PACKAGE)
        .cloned()
}

/// im 在跑（`extension.status` 里它那一项是 `running`）、状态文件的进程号对得上的，状态文件写着的页面服务的端口。
pub(crate) fn page_port(status: &Value, board: Option<&Value>) -> Option<u16> {
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

#[cfg(test)]
mod tests;

/// 印一行；印不出来也没有别处可说了。
fn say(to: &mut dyn Write, line: &str) {
    if writeln!(to, "{line}").is_err() {
        // 标准输出、标准错误关了：没有别处可说。
    }
}
