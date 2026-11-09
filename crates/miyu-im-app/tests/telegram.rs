//! Telegram 驱动的骨架（施工 I-183 中上，设计 07 第 3.5 节）：真核心拉起真程序，配置写一个机器人令牌、Bot API 指本机的假服务器
//! （`support::bot_api`）。连上了报号、名字、用户名、隐私模式；长轮询收到第几条记下，重启以后照它接着收；409 报「别处也在收」、
//! 401 报「令牌失效」，都不算连着。I-183 中下：私聊她，她回的字经 `sendMessage` 发回那个聊天。

use std::time::Duration;

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::bot_api::{BOT_ID, BotApi, Mode};
use crate::support::browser::{Channel, call, logged_in};
use crate::support::head::Head;
use crate::support::ports::on_free_port;
use crate::support::spawning::{bridge_up, cli, ports_config_with, text};
use crate::support::*;

/// 起一个核心和桥：Telegram 的连接表一个令牌，Bot API 是 `api`。交回数据根和登录好的 im 网页通道（问 Telegram 的状态用：
/// 经核心转的 `status` 只说第一个驱动的，网页的 `status` 认 `platform`）。
async fn started(api: &BotApi) -> (Home, Channel) {
    started_with(api, Script::new([Play::Says("在")])).await
}

/// 同 [`started`]，模型照 `script` 答。
async fn started_with(api: &BotApi, script: Script) -> (Home, Channel) {
    started_with_cards(api, script, None).await
}

/// 同 [`started_with`]，起来以前照 `cards` 写好卡片（`cards.json`）。
async fn started_with_cards(api: &BotApi, script: Script, cards: Option<&str>) -> (Home, Channel) {
    started_full(api, script, cards, "").await
}

/// 同 [`started_with_cards`]，主人对应表多 `bindings` 那几行（接在测试的对应表后面）。
async fn started_full(
    api: &BotApi,
    script: Script,
    cards: Option<&str>,
    bindings: &str,
) -> (Home, Channel) {
    let extra = format!(
        "tg.tokens = [{{ secret = \"tg\" }}]\ntg.api = \"{}\"",
        api.url
    );
    let home = on_free_port(async |listen| {
        let config = format!("{bindings}{}", ports_config_with(listen, &extra));
        let home = Home::spawning(&script, &config);
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
async fn until_tg(open: &mut Channel, wanted: impl Fn(&Value) -> bool) -> Value {
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

#[tokio::test]
async fn a_bot_connects_and_resumes_where_it_stopped_after_a_restart() {
    let api = BotApi::start(Mode::Normal).await;
    let (home, mut open) = started(&api).await;
    let connected = until_tg(&mut open, |one| one["connected"] == true).await;
    assert_eq!(
        connected,
        json!({"index": 0, "token": "set", "proxy": "", "connected": true, "bot": BOT_ID.to_string(), "username": "miyu_test_bot", "name": "Miyu Bot", "privacy": true})
    );
    // 长轮询：Telegram 挂 50 秒（`timeout`），只要消息。
    let first = api.until(|method, _| method == "getUpdates").await;
    assert_eq!(
        first,
        json!({"offset": 0, "timeout": 50, "allowed_updates": ["message"]})
    );
    api.push(json!({"update_id": 41, "message": {"message_id": 1, "date": 1, "chat": {"id": 5, "type": "private"}, "text": "hi"}}));
    api.until(|method, params| method == "getUpdates" && params["offset"] == 42)
        .await;

    // 重启：照记下的接着收，不重收第 41 条。
    let before = api.calls().len();
    let restarted = cli(&home.root, &["restart"]).await;
    assert_eq!(
        restarted.status.code(),
        Some(0),
        "{}",
        text(&restarted.stderr)
    );
    let polled = api
        .until_from(before, |method, _| method == "getUpdates")
        .await;
    assert_eq!(polled["offset"], 42, "重启以后照记下的接着收");
    home.stop_extensions().await;
}

#[tokio::test]
async fn a_token_used_elsewhere_is_reported_and_not_fought_over() {
    let api = BotApi::start(Mode::Conflict).await;
    let (home, mut open) = started(&api).await;
    let elsewhere = until_tg(&mut open, |one| one["trouble"] == "elsewhere").await;
    assert_eq!(elsewhere["connected"], false, "{elsewhere}");
    assert_eq!(
        elsewhere["bot"],
        BOT_ID.to_string(),
        "号照 getMe 认出来了：{elsewhere}"
    );
    // 不抢：409 以后隔一阵才再试，不会接着一直问。
    tokio::time::sleep(Duration::from_secs(1)).await;
    let tries = api
        .calls()
        .iter()
        .filter(|(method, _)| method == "getUpdates")
        .count();
    assert_eq!(tries, 1, "{:?}", api.calls());
    home.stop_extensions().await;
}

#[tokio::test]
async fn a_refused_token_is_reported_and_not_retried() {
    let api = BotApi::start(Mode::Unauthorized).await;
    let (home, mut open) = started(&api).await;
    let refused = until_tg(&mut open, |one| one["trouble"] == "token").await;
    assert_eq!(
        refused,
        json!({"index": 0, "token": "set", "proxy": "", "connected": false, "trouble": "token"})
    );
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(api.calls().len(), 1, "401 不再试：{:?}", api.calls());
    let log = std::fs::read_to_string(miyu_im::dirs::log_file(&home.root)).expect("写了运行日志");
    assert!(!log.contains(TG_TOKEN), "运行日志不记令牌：{log}");
    home.stop_extensions().await;
}

#[tokio::test]
async fn someone_talks_to_her_in_private_and_she_answers_there() {
    let api = BotApi::start(Mode::Normal).await;
    // Telegram 的「其他私聊」打开：不是主人、不在白名单里的人私聊也回。
    let cards = r#"{"others": {"tg": {"privates": {"on": true}}}}"#;
    let (home, mut open) =
        started_with_cards(&api, Script::new([Play::Says("在呢")]), Some(cards)).await;
    until_tg(&mut open, |one| one["connected"] == true).await;
    api.push(json!({"update_id": 1, "message": {
        "message_id": 7, "date": 1_700_000_000,
        "chat": {"id": ADMIN, "type": "private"},
        "from": {"id": ADMIN, "is_bot": false, "first_name": "Shorin"},
        "text": "你好",
    }}));
    let sent = api.until(|method, _| method == "sendMessage").await;
    assert_eq!(sent["chat_id"], ADMIN.to_string(), "{sent}");
    assert_eq!(sent["text"], "在呢", "{sent}");
    assert!(
        sent.get("message_thread_id").is_none(),
        "私聊没有话题：{sent}"
    );
    home.stop_extensions().await;
}

#[tokio::test]
async fn called_in_a_topic_she_answers_in_that_topic() {
    let api = BotApi::start(Mode::Normal).await;
    // Telegram 的「其他群」打开、每一步都发（同 QQ 那边群测试的老默认）。
    let cards = r#"{"others": {"tg": {"groups": {"on": true, "final_only": false}}}}"#;
    let (home, mut open) =
        started_with_cards(&api, Script::new([Play::Says("在呢")]), Some(cards)).await;
    until_tg(&mut open, |one| one["connected"] == true).await;
    let text = "@miyu_test_bot 在吗";
    api.push(json!({"update_id": 1, "message": {
        "message_id": 50, "date": 1_700_000_000,
        "chat": {"id": -1001, "type": "supergroup", "title": "Miyu 测试群", "is_forum": true},
        "from": {"id": ADMIN, "is_bot": false, "first_name": "Shorin"},
        "is_topic_message": true, "message_thread_id": 3,
        "reply_to_message": {"message_id": 3, "from": {"id": ADMIN}},
        "text": text, "entities": [{"type": "mention", "offset": 0, "length": 14}],
    }}));
    let sent = api.until(|method, _| method == "sendMessage").await;
    assert_eq!(sent["chat_id"], "-1001", "{sent}");
    assert_eq!(sent["message_thread_id"], 3, "回在原来的话题里：{sent}");
    assert!(
        sent["text"]
            .as_str()
            .is_some_and(|said| said.ends_with("在呢")),
        "{sent}"
    );
    home.stop_extensions().await;
}

#[tokio::test]
async fn the_owner_gets_the_command_menu_in_private() {
    // 施工 I-183 下上（项目主人 11-09）：号连上时 im 问核心有哪些命令、谁是主人，只给主人的私聊登记命令菜单。
    let api = BotApi::start(Mode::Normal).await;
    let owner = "\"tg:10001\" = \"owner\"\n";
    let (home, _open) = started_full(&api, Script::new([Play::Says("在")]), None, owner).await;
    let menu = api.until(|method, _| method == "setMyCommands").await;
    assert_eq!(
        menu["scope"],
        json!({"type": "chat", "chat_id": 10001}),
        "{menu}"
    );
    let commands = menu["commands"].as_array().expect("一列命令");
    assert!(!commands.is_empty(), "{menu}");
    for command in commands {
        let name = command["command"].as_str().expect("有名字");
        assert!(
            !name.is_empty()
                && name.len() <= 32
                && name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
            "照 Telegram 的写法：{command}"
        );
    }
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let menus = api
        .calls()
        .iter()
        .filter(|(method, _)| method == "setMyCommands")
        .count();
    assert_eq!(menus, 1, "只给主人一个人登记：{:?}", api.calls());
    home.stop_extensions().await;
}
