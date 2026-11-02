//! 打开主网页（施工 I-110，项目主人 11-02：装了网页软件的，主网页能跳到 im 网页，im 网页却跳不回去）：网页那头认 `#ticket=<票>`
//! （网页的 `served.js`，`miyu web` 也这样开），票是这份登录的人向核心要的一次性交接票（`login.ticket`，60 秒作废）。
//!
//! - `web.status` → `{"installed"}`：核心的包列表里有没有网页软件（[`WEBUI`]）。侧栏照它决定放不放入口。
//! - `web.open` → `{"url"}`：`http://127.0.0.1:<web.port>/#ticket=<票>`。没装网页的回 `not_installed`；读不到端口的回 `no_port`。
//!   im 网页只听本机，主网页也照本机地址开。

use serde_json::{Value, json};

use super::Person;
use super::plugins::core;

/// 网页软件的包编号。
const WEBUI: &str = "miyu-webui";

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
pub(super) async fn open(person: &Person) -> Result<Value, (String, String)> {
    if !installed(person).await? {
        return Err(("not_installed".to_string(), String::new()));
    }
    let got = core(person, "config.get", json!({"keys": [PORT]})).await?;
    let port = got["items"][PORT]["value"]
        .as_u64()
        .filter(|port| (1..=65535).contains(port))
        .ok_or_else(|| ("no_port".to_string(), String::new()))?;
    let ticket = core(person, "login.ticket", json!({})).await?;
    let ticket = ticket["ticket"]
        .as_str()
        .ok_or_else(|| ("bad_reply".to_string(), String::new()))?;
    Ok(json!({"url": url(port, ticket)}))
}

/// 主网页带交接票的地址（纯逻辑）。
fn url(port: u64, ticket: &str) -> String {
    format!("http://127.0.0.1:{port}/#ticket={ticket}")
}

#[cfg(test)]
mod tests {
    use super::url;

    #[test]
    fn the_ticket_rides_in_the_fragment() {
        assert_eq!(url(8310, "ab12"), "http://127.0.0.1:8310/#ticket=ab12");
    }
}
