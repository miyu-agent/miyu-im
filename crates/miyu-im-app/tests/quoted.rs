//! 引用的那一条的内容（施工 I-238，核心 QT-1）：真核心拉起真桥，群里一条引用了别的一条，桥交以前取回那一条，写进 `venue.quoted`。
//!
//! - QQ：照 `get_msg` 取，名字照 `sender`、里面的转发照样展开、带的东西照那一条自己的编号（施工 I-245）；近况里写在 `reply-to`
//!   下面，她的日志里有那一条的也写（核心 QT-2）。
//! - 取不到的、不是这个群的不写，记一行运行日志，消息照记。
//! - Telegram：照收的时候 `reply_to_message` 带着的那一整条，不另问。

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::bot_api::{BotApi, Mode};
use crate::support::events::run_log;
use crate::support::group::*;
use crate::support::media::*;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};
use crate::support::telegram::{self as tg, ALICE, WHEN_CALLED, started_ruled, until_tg};
use crate::support::*;

/// 群 `group` 里号是 `user`、群名片是 `card` 的人说的一条（`get_msg` 回的 `data`）。
pub fn posted(group: i64, user: i64, card: &str, message: Value) -> Value {
    json!({"message_type": "group", "group_id": group, "user_id": user,
        "sender": {"user_id": user, "card": card, "nickname": "nick"}, "message": message})
}

/// 群里第 `msg` 条记下的场所的格。
pub fn recorded(home: &Home, venue: &str, msg: &str) -> Value {
    said(&venue_events(&home.root, venue))
        .into_iter()
        .find(|event| event["body"]["venue"]["msg"] == msg)
        .map(|event| event["body"]["venue"].clone())
        .expect("记下了")
}

#[tokio::test]
async fn a_quote_she_never_saw_is_fetched_and_shown_under_reply_to() {
    let lines = Lines::new([Line::says("周六见。")]);
    let (home, napcat) = up(&lines).await;
    let inner = json!({"type": "forward", "data": {"id": "fw-q", "content": [
        {"message_id": 1, "user_id": 20003, "time": TIME, "message_type": "group",
         "sender": {"user_id": 20003, "card": "小美", "nickname": ""}, "message": [plain("路线在这"), image("r.jpg", 1024)]},
    ]}});
    napcat.stock_message(
        950,
        posted(
            GROUP,
            20001,
            "阿杰",
            json!([plain("周六见"), image("p.jpg", 2048), inner]),
        ),
    );
    says(
        &home,
        &napcat,
        (MEMBER, "member"),
        951,
        json!([quote(950), plain("好")]),
    )
    .await;
    assert_eq!(
        recorded(&home, &venue(), "951")["quoted"],
        json!({"name": "阿杰", "text": "周六见[forward]",
            "media": [{"kind": "image", "id": "p.jpg", "size": 2048, "index": 1}],
            "forwards": [{"nodes": [{"name": "小美", "text": "路线在这",
                "media": [{"kind": "image", "id": "r.jpg", "size": 1024, "index": 2}]}]}],
        }),
        "取回来、里面的转发照样展开；带的东西照那一条自己的编号（施工 I-245：她照 msg=950 取得到）"
    );
    says(
        &home,
        &napcat,
        (ADMIN, "member"),
        952,
        json!([at(BOT), plain(" 看看")]),
    )
    .await;
    results(&home, 1).await;
    let text = user_text(&lines.requests()[0]);
    assert!(
        text.contains(concat!(
            "  reply-to: msg=950\n",
            "    阿杰: 周六见[forward] [image #1: 2 KB]\n",
            "      forward:\n",
            "        小美: 路线在这 [image #2: 1 KB]",
        )),
        "她没见过的写在 reply-to 下面：{text}"
    );
    stopped(home).await;
}

#[tokio::test]
async fn a_quote_in_her_log_is_written_out_too_and_one_missing_or_from_elsewhere_is_left_out() {
    let lines = Lines::new([Line::says("嗯。")]);
    let (home, napcat) = up(&lines).await;
    let words = json!([plain("今天谁值班")]);
    napcat.stock_message(960, posted(GROUP, MEMBER, "小明", words.clone()));
    napcat.stock_message(
        980,
        posted(
            999,
            MEMBER,
            "小明",
            json!([plain("别的群"), image("x.jpg", 1024)]),
        ),
    );
    says(&home, &napcat, (MEMBER, "member"), 960, words).await;
    for (msg, quoted) in [(961, 960), (962, 970), (963, 980)] {
        let words = json!([quote(quoted), plain("我")]);
        says(&home, &napcat, (ADMIN, "member"), msg, words).await;
    }
    let quoted = |msg: &str| recorded(&home, &venue(), msg).get("quoted").cloned();
    assert_eq!(
        quoted("961"),
        Some(json!({"name": "小明", "text": "今天谁值班"})),
        "有引用就取：在不在她那里由核心判"
    );
    assert_eq!(
        (quoted("962"), quoted("963")),
        (None, None),
        "取不到的、别的群的不写"
    );
    let log = run_log(&home.root);
    for why in ["why=failed", "why=elsewhere"] {
        assert!(
            log.lines()
                .any(|line| line.contains("quote not fetched") && line.contains(why)),
            "{why}：{log}"
        );
    }
    says(
        &home,
        &napcat,
        (ADMIN, "member"),
        964,
        json!([quote(980), at(BOT), plain(" 在吗")]),
    )
    .await;
    results(&home, 1).await;
    let text = user_text(&lines.requests()[0]);
    assert!(
        text.contains("  reply-to: msg=960\n    小明: 今天谁值班\n"),
        "日志里有的也写原文（核心 QT-2，2026-11-16 项目主人定）：{text}"
    );
    assert!(!text.contains("别的群"), "{text}");
    // 冲她来的这一条另取引用的那一条的原图（施工 I-245）：别的群的不取。
    let log = run_log(&home.root);
    assert!(
        log.lines().any(|line| line.contains("picture not fetched")
            && line.contains("message=980")
            && line.contains("why=elsewhere")),
        "{log}"
    );
    assert!(
        !napcat
            .fetched()
            .iter()
            .any(|(action, _)| action == "get_image"),
        "别的群的图不取"
    );
    stopped(home).await;
}

#[tokio::test]
async fn on_telegram_the_quote_comes_with_the_update() {
    let api = BotApi::start(Mode::Normal).await;
    let script = Script::new([Play::Says("对")]);
    let cards = r#"{"others": {"tg": {"groups": {"on": true, "final_only": false}}}}"#;
    let (home, mut open) =
        started_ruled(&api, script.clone(), (Some(cards), ""), WHEN_CALLED).await;
    until_tg(&mut open, |one| one["connected"] == true).await;
    let chat = json!({"id": tg::GROUP, "type": "supergroup", "title": "Miyu 测试群"});
    // 引用的那一条桥没收到过（比桥连上早）：只在这一条的 `reply_to_message` 里。
    let earlier = json!({
        "message_id": 70, "date": 1_700_000_070, "chat": chat,
        "from": {"id": ALICE, "is_bot": false, "first_name": "Alice"},
        "text": "周六见",
    });
    api.push(json!({"update_id": 1, "message": {
        "message_id": 71, "date": 1_700_000_071, "chat": chat,
        "from": {"id": ADMIN, "is_bot": false, "first_name": "Shorin"},
        "text": "@miyu_test_bot 对吗", "entities": [{"type": "mention", "offset": 0, "length": 14}],
        "reply_to_message": earlier,
    }}));
    api.until(|method, params| method == "sendMessage" && params["text"] == "对")
        .await;
    assert_eq!(
        recorded(&home, &tg::venue(), "71")["quoted"],
        json!({"name": "Alice", "text": "周六见"})
    );
    let (_, request) = script.requests().into_iter().next().expect("请求过模型");
    let text = user_text(&request);
    assert!(
        text.contains("  reply-to: msg=70\n    Alice: 周六见"),
        "{text}"
    );
    home.stop_extensions().await;
}
