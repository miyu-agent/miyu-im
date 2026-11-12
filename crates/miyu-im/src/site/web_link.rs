//! 打开主网页（施工 I-110，项目主人 11-02：装了网页软件的，主网页能跳到 im 网页，im 网页却跳不回去）：网页那头认 `#ticket=<票>`
//! （网页的 `served.js`，`miyu web` 也这样开），票是这份登录的人向核心要的一次性交接票（`login.ticket`，60 秒作废）。
//!
//! - `web.status` → `{"installed"}`：核心的包列表里有没有网页软件（[`WEBUI`]）。侧栏照它决定放不放入口。
//! - `web.open {session?}` → `{"url"}`：主网页真正在听的地址加 `#ticket=<票>`；带了 `session` 的再加 `&session=<编号>`，网页软件直接
//!   打开那个会话（施工 I-232，todolist 第 59 条：会话设置卡片的「查看会话」，场所会话在网页那边只读）。地址照 `miyu web --print` 印的（施工 I-201，项目主人 11-10：
//!   原来照系统配置的 `web.port` 拼 `http://127.0.0.1:<端口>`，网页不在那个端口上听的时候跳错地方；`miyu web` 认的是网页软件自己记下
//!   的地址，没在跑的还会先拉起来）。印不出来的（主程序找不到、十几秒没回）退回照 `web.port` 拼。没装网页的回 `not_installed`；
//!   两样都拿不到的回 `no_port`。

use std::time::Duration;

use serde_json::{Value, json};

use super::Person;
use super::plugins::core;

/// 网页软件的包编号（11-12 起不带 miyu-，网页那边 eba24c5d 改的，施工 I-228）。
const WEBUI: &str = "webui";

/// 网页的端口：系统配置的键。
const PORT: &str = "web.port";

/// 装没装网页软件。
async fn installed(person: &Person) -> Result<bool, (String, String)> {
    let listed = core(person, "package.list", json!({})).await?;
    Ok(listed["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|entry| entry["package"] == WEBUI))
}

/// `web.status`（模块的说明）。
pub(super) async fn status(person: &Person) -> Result<Value, (String, String)> {
    Ok(json!({"installed": installed(person).await?}))
}

/// `web.open`（模块的说明）。
pub(super) async fn open(person: &Person, params: &Value) -> Result<Value, (String, String)> {
    let session = match &params["session"] {
        Value::Null => None,
        Value::String(session) if is_session(session) => Some(session.clone()),
        _ => return Err(("bad_params".to_string(), String::new())),
    };
    if !installed(person).await? {
        return Err(("not_installed".to_string(), String::new()));
    }
    let origin = match printed().await {
        Some(origin) => origin,
        None => {
            let got = core(person, "config.get", json!({"keys": [PORT]})).await?;
            let port = got["items"][PORT]["value"]
                .as_u64()
                .filter(|port| (1..=65535).contains(port))
                .ok_or_else(|| ("no_port".to_string(), String::new()))?;
            format!("http://127.0.0.1:{port}")
        }
    };
    let ticket = core(person, "login.ticket", json!({})).await?;
    let ticket = ticket["ticket"]
        .as_str()
        .ok_or_else(|| ("bad_reply".to_string(), String::new()))?;
    Ok(json!({"url": url(&origin, ticket, session.as_deref())}))
}

/// 等 `miyu web --print` 最多多久：它自己拉起网页软件最多等 10 秒。
const WAIT: Duration = Duration::from_secs(15);

/// 跑一次 `miyu web --print`，交回它印的地址的开头（[`origin`]）；跑不成、到时没回、印的不是地址的是空的。
async fn printed() -> Option<String> {
    let run = tokio::task::spawn_blocking(|| {
        std::process::Command::new(crate::program::main_program())
            .args(["web", "--print"])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
    });
    let output = tokio::time::timeout(WAIT, run).await.ok()?.ok()?.ok()?;
    if !output.status.success() {
        return None;
    }
    origin(&String::from_utf8_lossy(&output.stdout))
}

/// `miyu web --print` 印的 `text` 里的地址，去掉 `#` 后面的（第一次用时带的一次性码）和末尾的 `/`（纯逻辑）。
fn origin(text: &str) -> Option<String> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("http://") || line.starts_with("https://"))?;
    let address = line.split('#').next().unwrap_or(line).trim_end_matches('/');
    (address.len() > "http://".len()).then(|| address.to_string())
}

/// 主网页带交接票的地址（纯逻辑）：`origin` 是不带末尾 `/` 的地址；带了会话 `session` 的直接打开它（同 `miyu web --session`）。
fn url(origin: &str, ticket: &str, session: Option<&str>) -> String {
    match session {
        Some(session) => format!("{origin}/#ticket={ticket}&session={session}"),
        None => format!("{origin}/#ticket={ticket}"),
    }
}

/// 会话编号的写法（纯逻辑）：字母、数字、`-`，不长于 64 个字（放进网址不用转义，同网页软件读参数时查的）。
fn is_session(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::{is_session, origin, url};

    #[test]
    fn the_ticket_rides_in_the_fragment() {
        assert_eq!(
            url("http://127.0.0.1:8310", "ab12", None),
            "http://127.0.0.1:8310/#ticket=ab12"
        );
    }

    #[test]
    fn a_session_rides_after_the_ticket_and_odd_ones_are_refused() {
        // 施工 I-232：「查看会话」带上会话编号，网页软件直接打开它。
        let session = "019a4b2c-7d3e-7f10-8a2b-3c4d5e6f7a8b";
        assert_eq!(
            url("http://127.0.0.1:8310", "ab12", Some(session)),
            format!("http://127.0.0.1:8310/#ticket=ab12&session={session}")
        );
        assert!(is_session(session));
        for odd in ["", "a&settings=x", "a#b", "会话", &"a".repeat(65)] {
            assert!(!is_session(odd), "{odd}");
        }
    }

    #[test]
    fn the_printed_address_is_taken_without_its_fragment() {
        // 施工 I-201：照 `miyu web --print` 印的地址开，不照配置的端口猜。
        assert_eq!(
            origin("http://127.0.0.1:48300/\n").as_deref(),
            Some("http://127.0.0.1:48300")
        );
        assert_eq!(
            origin("http://192.168.1.5:8300/#setup=abcd\n").as_deref(),
            Some("http://192.168.1.5:8300"),
            "第一次用时带的一次性码不要"
        );
        assert_eq!(origin("网页打不开\n"), None);
        assert_eq!(origin("http://\n"), None);
        assert_eq!(origin(""), None);
    }
}
