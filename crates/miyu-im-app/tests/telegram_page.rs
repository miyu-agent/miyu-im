//! Telegram 的连接在页面上新建、改、删（施工 I-183 下中，设计 07 第八节）：真核心拉起真程序，Bot API 指本机的假服务器，经 im 网页的通道
//! 走一遍：令牌不对的说 `bad_token`；对的查到机器人、存成密钥、连接表多一项，驱动照它连上，连接卡带用户名；重新检查交隐私模式；改代理；
//! 删了连接表空了。

use std::time::Duration;

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::bot_api::{BOT_ID, BotApi, Mode};
use crate::support::browser::{Channel, call, logged_in};
use crate::support::head::Head;
use crate::support::ports::on_free_port;
use crate::support::spawning::{bridge_up, cli, ports_config_with, text};
use crate::support::*;

/// 起一个核心和桥：Telegram 的连接表空着，Bot API 是 `api`；交回登录好的 im 网页通道。
async fn started(api: &BotApi) -> (Home, Channel) {
    let script = Script::new([Play::Says("在")]);
    let extra = format!("tg.api = \"{}\"", api.url);
    let home = on_free_port(async |listen| {
        let home = Home::spawning(&script, &ports_config_with(listen, &extra));
        let started = cli(&home.root, &["start"]).await;
        assert_eq!(started.status.code(), Some(0), "{}", text(&started.stderr));
        bridge_up(&home.root, listen).await?;
        Ok(home)
    })
    .await;
    let mut head = Head::connect(&home.root).await;
    let status = head.until("status", |result| result["web"].is_u64()).await;
    let web = u16::try_from(status["web"].as_u64().expect("有端口")).expect("是端口");
    let open = logged_in(&home.root, web).await;
    (home, open)
}

/// 问 `connections`，直到 Telegram 的卡合 `wanted`：交回那一张。
async fn until_card(open: &mut Channel, id: &mut u64, wanted: impl Fn(&Value) -> bool) -> Value {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        *id += 1;
        let reply = call(open, *id, "connections", json!({})).await;
        let found = reply["result"]["connections"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|card| card["platform"] == "tg" && wanted(card))
            .cloned();
        if let Some(card) = found {
            return card;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "等不到 Telegram 的连接卡：{reply}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn a_telegram_connection_is_added_checked_changed_and_removed_from_the_page() {
    let api = BotApi::start(Mode::Normal).await;
    let (home, mut open) = started(&api).await;
    let mut id = 1;
    // 令牌不对：Telegram 回 401，什么都不存。
    let wrong = call(
        &mut open,
        2,
        "connection.add",
        json!({"platform": "tg", "token": "1:wrong", "proxy": ""}),
    )
    .await;
    assert_eq!(wrong["error"]["reason"], "bad_token", "{wrong}");
    // 对的：查到的交回，存进连接表。
    let added = call(
        &mut open,
        3,
        "connection.add",
        json!({"platform": "tg", "token": TG_TOKEN, "proxy": "direct"}),
    )
    .await;
    assert_eq!(
        added["result"],
        json!({"saved": true, "bot": BOT_ID.to_string(), "name": "Miyu Bot", "username": "miyu_test_bot", "privacy": true}),
        "{added}"
    );
    // 驱动照新的连接表连上：卡上带用户名、隐私模式开着的毛病。
    let card = until_card(&mut open, &mut id, |card| card["connected"] == true).await;
    assert_eq!(card["username"], "miyu_test_bot", "{card}");
    assert_eq!(card["trouble"], "privacy", "{card}");
    // 同一个机器人再加：不收。
    let again = call(
        &mut open,
        50,
        "connection.add",
        json!({"platform": "tg", "token": TG_TOKEN, "proxy": ""}),
    )
    .await;
    assert_eq!(again["error"]["reason"], "bot_taken", "{again}");
    // 重新检查：照表里那一个现在的令牌再查。
    let checked = call(
        &mut open,
        51,
        "connection.check",
        json!({"platform": "tg", "bot": BOT_ID.to_string()}),
    )
    .await;
    assert_eq!(checked["result"]["privacy"], true, "{checked}");
    // 改代理：连接表里的那一格换上，驱动换一个连接再连上。
    let edited = call(
        &mut open,
        52,
        "connection.edit",
        json!({"platform": "tg", "bot": BOT_ID.to_string(), "values": {"proxy": "  "}}),
    )
    .await;
    assert_eq!(edited["result"], json!({"saved": true}), "{edited}");
    let status = call(&mut open, 53, "status", json!({"platform": "tg"})).await;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut proxy = status["result"]["connections"][0]["proxy"].clone();
    let mut asked = 60;
    while proxy != "env" {
        assert!(
            tokio::time::Instant::now() < deadline,
            "代理没换上：{status}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
        asked += 1;
        proxy = call(&mut open, asked, "status", json!({"platform": "tg"})).await["result"]["connections"][0]["proxy"].clone();
    }
    // 删掉：连接表空了，卡没了。
    let removed = call(
        &mut open,
        200,
        "connection.remove",
        json!({"platform": "tg", "bot": BOT_ID.to_string()}),
    )
    .await;
    assert_eq!(removed["result"], json!({"saved": true}), "{removed}");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut asked = 300;
    loop {
        asked += 1;
        let status = call(&mut open, asked, "status", json!({"platform": "tg"})).await;
        if status["result"]["connections"] == json!([]) {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "连接表没空：{status}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    home.stop_extensions().await;
}
