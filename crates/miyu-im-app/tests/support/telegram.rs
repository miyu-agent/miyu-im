//! 起 Telegram 的测试（施工 I-183 中上起在 `telegram.rs`，I-183 下下挪来给工具的测试同用）：真核心拉起真程序，配置写一个机器人令牌、
//! Bot API 指本机的假服务器（[`super::bot_api`]）。

use std::time::Duration;

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use super::Home;
use super::bot_api::{BOT_ID, BotApi};
use super::browser::{Channel, call, logged_in};
use super::head::Head;
use super::ports::on_free_port;
use super::spawning::{bridge_up, cli, ports_config_with, text};

/// 起一个核心和桥：Telegram 的连接表一个令牌，Bot API 是 `api`。交回数据根和登录好的 im 网页通道（问 Telegram 的状态用：
/// 经核心转的 `status` 只说第一个驱动的，网页的 `status` 认 `platform`）。
pub async fn started(api: &BotApi) -> (Home, Channel) {
    started_with(api, Script::new([Play::Says("在")])).await
}

/// 同 [`started`]，模型照 `script` 答。
pub async fn started_with(api: &BotApi, script: Script) -> (Home, Channel) {
    started_with_cards(api, script, None).await
}

/// 同 [`started_with`]，起来以前照 `cards` 写好卡片（`cards.json`）。
pub async fn started_with_cards(
    api: &BotApi,
    script: Script,
    cards: Option<&str>,
) -> (Home, Channel) {
    started_full(api, script, cards, "").await
}

/// 同 [`started_with_cards`]，主人对应表多 `bindings` 那几行（接在测试的对应表后面）。
pub async fn started_full(
    api: &BotApi,
    script: Script,
    cards: Option<&str>,
    bindings: &str,
) -> (Home, Channel) {
    started_ruled(api, script, (cards, bindings), "").await
}

/// 同 [`started_full`]，场所规则多一份 `rules`（写进 `venues.d/80-test.toml`；空的不写）。
pub async fn started_ruled(
    api: &BotApi,
    script: Script,
    (cards, bindings): (Option<&str>, &str),
    rules: &str,
) -> (Home, Channel) {
    let extra = format!(
        "tg.tokens = [{{ secret = \"tg\" }}]\ntg.api = \"{}\"",
        api.url
    );
    let home = on_free_port(async |listen| {
        let config = format!("{bindings}{}", ports_config_with(listen, &extra));
        let home = Home::spawning(&script, &config);
        if !rules.is_empty() {
            let dir = miyu_im::dirs::config(&home.root).join("venues.d");
            std::fs::create_dir_all(&dir).expect("建得了目录");
            std::fs::write(dir.join("80-test.toml"), rules).expect("写得进");
        }
        if let Some(cards) = cards {
            let dir = miyu_im::dirs::state(&home.root);
            std::fs::create_dir_all(&dir).expect("建得了目录");
            std::fs::write(dir.join("cards.json"), cards).expect("写得进");
        }
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

/// 问 Telegram 那个驱动的状态，直到头一个连接合 `wanted`：交回那一个连接。
pub async fn until_tg(open: &mut Channel, wanted: impl Fn(&Value) -> bool) -> Value {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    let mut id = 100;
    loop {
        id += 1;
        let reply = call(open, id, "status", json!({"platform": "tg"})).await;
        let first = &reply["result"]["connections"][0];
        if wanted(first) {
            return first.clone();
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "等不到 Telegram 的状态：{reply}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// 测试群。
pub const GROUP: i64 = -1001;

/// 群里说过话的人（没有用户名）。
pub const ALICE: i64 = 20001;

/// 群里叫她才回（不过判官）：别人随口说的只记下。
pub const WHEN_CALLED: &str =
    "[[rule]]\nmatch = { kind = \"group\" }\ndiscipline = \"when-called\"\n";

/// 测试群的场所。
pub fn venue() -> String {
    format!("tg:{BOT_ID}:group:{GROUP}")
}

/// 群里的一条：编号 `id`、谁说的、说什么；`called` 的开头 @ 她。
pub fn said(update: i64, id: i64, (from, name): (i64, &str), text: &str, called: bool) -> Value {
    let mut message = json!({
        "message_id": id, "date": 1_700_000_000 + id,
        "chat": {"id": GROUP, "type": "supergroup", "title": "Miyu 测试群"},
        "from": {"id": from, "is_bot": false, "first_name": name},
        "text": text,
    });
    if called {
        message["entities"] = json!([{"type": "mention", "offset": 0, "length": 14}]);
    }
    json!({"update_id": update, "message": message})
}
