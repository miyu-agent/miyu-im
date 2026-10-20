//! 插件面板（施工 I-9 下，设计 01 第四节第 5 条、第九节第 5 条；照核心 `package-pages.md`「网页软件怎么给后台页」、WebUI 的
//! `backstage.rs`）：有后台页（清单 `[page]`）的插件，页面在沙箱框里打开它。
//!
//! - `plugins.panel {package}` → `{"url": "/p/<票据>/<编号>/"}`：给这份登录、这个插件造一张票据（随机 32 字节写成十六进制）。不是 im
//!   插件的 `bad_params`，没有后台页的 `no_page`。票据最多留 [`HELD`] 张，多了拿掉最早的；登出、登录被挤掉以后它对不上任何登录，没用了。
//! - `GET /p/<票据>/<编号>/<路径>`：票据对上、编号对上的，在那份登录的人的连接上照 `package.file` 一块块读完（一份最多 [`MOST`]
//!   字节）交给 `request` 回；路径的 `%xx` 换回来，空的、以 `/` 结尾的补上 `index.html`（核心照它的规矩查路径）。对不上、读不成的 404。
//!   框里的页面在空的来源里跑（不给 `allow-same-origin`），回应头照 `request::framed`。

use std::sync::{Mutex, MutexGuard, PoisonError};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use super::logins::{Login, same, session};
use super::plugins::{bad, core, is_plugin};
use super::{Person, Site, files};
use crate::TARGET;

/// 最多留几张票据。
pub(super) const HELD: usize = 32;

/// 一份文件最多多少字节。
const MOST: usize = 32 * 1024 * 1024;

/// 一张票据：哪份登录（会话号）、哪个插件。
struct Ticket {
    id: String,
    session: String,
    package: String,
}

/// 发出去的票据，照先后。
#[derive(Default)]
pub(crate) struct Panels {
    held: Mutex<Vec<Ticket>>,
}

impl Panels {
    /// 给会话号 `session` 的登录、插件 `package` 造一张票据；取不到随机数的是空的。
    pub(super) fn open(&self, session_id: &str, package: &str) -> Option<String> {
        let id = session()?;
        let mut held = self.lock();
        held.push(Ticket {
            id: id.clone(),
            session: session_id.to_string(),
            package: package.to_string(),
        });
        if held.len() > HELD {
            held.remove(0);
        }
        Some(id)
    }

    /// 票据 `id` 是哪份登录（会话号）、哪个插件；没有的是空的。每一张都按常数时间比完。
    pub(super) fn find(&self, id: &str) -> Option<(String, String)> {
        let held = self.lock();
        let mut found = None;
        for ticket in held.iter() {
            if same(ticket.id.as_bytes(), id.as_bytes()) {
                found = Some((ticket.session.clone(), ticket.package.clone()));
            }
        }
        found
    }

    /// 锁里不 `await`；真崩了，里面的照样能用。
    fn lock(&self) -> MutexGuard<'_, Vec<Ticket>> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// `plugins.panel`（模块的说明）。
pub(super) async fn open(
    site: &Site,
    login: &Login<Person>,
    params: &Value,
) -> Result<Value, (String, String)> {
    let package = params["package"].as_str().ok_or_else(bad)?;
    if !is_plugin(&login.person, package).await? {
        return Err(bad());
    }
    let listed = core(&login.person, "package.list", json!({})).await?;
    let paged = listed["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|entry| entry["package"] == package)
        .is_some_and(|entry| {
            entry
                .get("page")
                .is_some_and(|page| !page.is_null() && *page != false)
        });
    if !paged {
        return Err(("no_page".to_string(), String::new()));
    }
    let ticket = site
        .panels
        .open(&login.session, package)
        .ok_or_else(|| ("unavailable".to_string(), String::new()))?;
    tracing::info!(target: TARGET, package, "plugin panel opened");
    Ok(json!({"url": format!("/p/{ticket}/{package}/")}))
}

/// `GET /p/…`（模块的说明）：交回这份文件和它的 `Content-Type`；对不上、读不成的是空的。
pub(super) async fn file(site: &Site, path: &str) -> Option<(Vec<u8>, &'static str)> {
    let (ticket, package, rest) = split(path)?;
    let (session_id, owner) = site.panels.find(&ticket)?;
    if owner != package {
        return None;
    }
    let login = site.logins.get(&session_id)?;
    let mut bytes = Vec::new();
    loop {
        let params = json!({"package": package, "path": rest, "offset": bytes.len()});
        let read = core(&login.person, "package.file", params).await;
        let got = match read {
            Ok(got) => got,
            Err((reason, _)) => {
                tracing::info!(target: TARGET, package, path = rest, reason, "plugin panel file not given");
                return None;
            }
        };
        bytes.extend(STANDARD.decode(got["data"].as_str()?).ok()?);
        if bytes.len() > MOST {
            return None;
        }
        if got["eof"].as_bool().unwrap_or(true) {
            return Some((bytes, files::kind(&rest)));
        }
    }
}

/// `/p/<票据>/<编号>/<路径>` 拆成三段：路径的 `%xx` 换回来，空的、以 `/` 结尾的补上 `index.html`。不是这个样子的是空的。
pub(super) fn split(path: &str) -> Option<(String, String, String)> {
    let mut parts = path.strip_prefix("/p/")?.splitn(3, '/');
    let ticket = parts.next().filter(|ticket| !ticket.is_empty())?;
    let package = parts.next().filter(|package| !package.is_empty())?;
    let mut rest = decode(parts.next().unwrap_or_default())?;
    if rest.is_empty() || rest.ends_with('/') {
        rest.push_str("index.html");
    }
    Some((ticket.to_string(), package.to_string(), rest))
}

/// 把 `%xx` 换回字节；写错的、换回来不是 UTF-8 的是空的。
fn decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let hex = std::str::from_utf8(bytes.get(at + 1..at + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests;
