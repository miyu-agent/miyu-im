//! 私聊里引用的那一条的内容（施工 I-245，核心 QT-2）：私聊里一条引用了别的一条，桥交以前取回那一条，写进 `venue.quoted`，核心
//! 在那一条的最后另起一块写 `reply-to: msg=…`，下面缩进写原文。
//!
//! - QQ：照 `get_msg` 取；对方说的、她自己说的都带，别的私聊的、群里的不带，记一行运行日志。
//! - Telegram：照收的时候 `reply_to_message` 带着的那一整条。

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::quoted::{posted, recorded};
use crate::support::bot_api::{BOT_ID, BotApi, Mode};
use crate::support::events::run_log;
use crate::support::group::*;
use crate::support::media::*;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};
use crate::support::telegram::{started_ruled, until_tg};
use crate::support::*;

/// 私聊里号是 `user`、昵称是 `nick` 的人说的一条（`get_msg` 回的 `data`）。
fn whispered(user: i64, nick: &str, message: Value) -> Value {
    json!({"message_type": "private", "user_id": user,
        "sender": {"user_id": user, "nickname": nick}, "message": message})
}

/// 终端管理员私聊里第 `msg` 条记下的场所的格（他本人的私聊不关在沙盒里，在他自己名下）；还没记下的是空的。
fn mine(home: &Home, msg: &str) -> Option<Value> {
    home.own_sessions()
        .iter()
        .flat_map(|session| home.events(session))
        .find(|event| event["kind"] == "message.user" && event["body"]["venue"]["msg"] == msg)
        .map(|event| event["body"]["venue"].clone())
}

/// 终端管理员私聊里说第 `msg` 条（`words` 是段），等它记下。
async fn whispers(home: &Home, napcat: &Answering, msg: i64, words: Value) {
    napcat.send(private_frame(ADMIN, msg, words));
    let id = msg.to_string();
    let deadline = tokio::time::Instant::now() + WAIT;
    while mine(home, &id).is_none() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "第 {id} 条等不到记下"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn in_a_private_chat_the_quote_comes_too_hers_included() {
    let lines = Lines::new([(); 4].map(|()| Line::says("嗯。")));
    let (home, napcat) = up(&lines).await;
    napcat.stock_message(990, whispered(ADMIN, "主人", json!([plain("周六见")])));
    napcat.stock_message(
        991,
        whispered(
            BOT,
            "Miyu",
            json!([plain("那我先走了"), image("s.jpg", 1024)]),
        ),
    );
    napcat.stock_message(992, whispered(STRANGER, "路人", json!([plain("别人的话")])));
    napcat.stock_message(
        993,
        posted(GROUP, MEMBER, "小明", json!([plain("群里的话")])),
    );
    for (msg, quoted) in [(994, 990), (995, 991), (996, 992), (997, 993)] {
        whispers(&home, &napcat, msg, json!([quote(quoted), plain("嗯")])).await;
    }
    let quoted = |msg: &str| mine(&home, msg).and_then(|venue| venue.get("quoted").cloned());
    assert_eq!(
        quoted("994"),
        Some(json!({"name": "主人", "text": "周六见"}))
    );
    assert_eq!(
        quoted("995"),
        Some(json!({"name": "Miyu", "text": "那我先走了",
            "media": [{"kind": "image", "id": "s.jpg", "size": 1024}]})),
        "她自己说的那一条也带；带的东西不编号：`fetch_media` 取不到她自己说的"
    );
    assert_eq!(
        (quoted("996"), quoted("997")),
        (None, None),
        "别的私聊的、群里的不带"
    );
    let log = run_log(&home.root);
    let elsewhere = log
        .lines()
        .filter(|line| line.contains("quote not fetched") && line.contains("why=elsewhere"))
        .count();
    assert_eq!(elsewhere, 2, "{log}");
    until_private_turns(&home, 1).await;
    let text = user_text(&lines.requests()[0]);
    assert!(
        text.contains("\nreply-to: msg=990\n  主人: 周六见"),
        "私聊也写在 reply-to 下面：{text}"
    );
    stopped(home).await;
}

#[tokio::test]
async fn on_telegram_a_private_quote_comes_with_the_update() {
    let api = BotApi::start(Mode::Normal).await;
    let script = Script::new([Play::Says("对")]);
    let cards = r#"{"others": {"tg": {"privates": {"on": true}}}}"#;
    let (home, mut open) = started_ruled(&api, script.clone(), (Some(cards), ""), "").await;
    until_tg(&mut open, |one| one["connected"] == true).await;
    let chat = json!({"id": ADMIN, "type": "private"});
    // 她自己先前说的一条（桥连上以前的）：只在这一条的 `reply_to_message` 里。
    let earlier = json!({
        "message_id": 80, "date": 1_700_000_080, "chat": chat,
        "from": {"id": BOT_ID, "is_bot": true, "first_name": "Miyu"},
        "text": "周六见",
    });
    api.push(json!({"update_id": 1, "message": {
        "message_id": 81, "date": 1_700_000_081, "chat": chat,
        "from": {"id": ADMIN, "is_bot": false, "first_name": "Shorin"},
        "text": "几点", "reply_to_message": earlier,
    }}));
    api.until(|method, params| method == "sendMessage" && params["text"] == "对")
        .await;
    let venue = format!("tg:{BOT_ID}:private:{ADMIN}");
    assert_eq!(
        recorded(&home, &venue, "81")["quoted"],
        json!({"name": "Miyu", "text": "周六见"})
    );
    let (_, request) = script.requests().into_iter().next().expect("请求过模型");
    let text = user_text(&request);
    assert!(
        text.contains("\nreply-to: msg=80\n  Miyu: 周六见"),
        "{text}"
    );
    home.stop_extensions().await;
}
