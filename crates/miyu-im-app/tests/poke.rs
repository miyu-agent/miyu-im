//! 戳一戳（施工 I-248，2026-11-17 项目主人定；核心 VP-1）：真核心拉起真桥、假 NapCat 推 `notice`/`notify`/`poke`。
//!
//! - 终端管理员私聊里戳她：当作他发来的一条（没有字，场所的格 `poke: true`、带后缀的写 `poke_text`），开一轮，她的回话发回私聊。
//! - 群里群友戳她：当作冲她来的一条照群里的规则判（这里叫她就回），开一轮；回话不引用、不贴表情（那一条平台上没有）。
//! - 群里戳别人：只记进聊天记录（一件 `poked`），不进会话、不开轮。

use serde_json::{Value, json};

use crate::support::group::*;
use crate::support::media::user_text;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 号是 `user` 的人戳了 `target`，群号 `group`（私聊的不写），灰条拆成的几段 `raw`。
fn poke(user: i64, target: i64, group: Option<i64>, raw: Value) -> Value {
    let mut frame = json!({"post_type": "notice", "notice_type": "notify", "sub_type": "poke", "time": TIME,
        "self_id": BOT, "user_id": user, "target_id": target, "raw_info": raw});
    if let Some(group) = group {
        frame["group_id"] = json!(group);
    }
    frame
}

/// 「戳了戳你的小脑袋」那条灰条。
fn nudge() -> Value {
    json!([
        {"col": "1", "nm": "", "type": "qq", "uid": "u_a"},
        {"txt": "戳了戳", "type": "nor"},
        {"col": "1", "nm": "", "tp": "0", "type": "qq", "uid": "u_b"},
        {"txt": "的小脑袋", "type": "nor"},
    ])
}

/// 下一个发给终端管理员的私聊的字（平台动作跳过）。
async fn next_private(napcat: &mut Answering) -> String {
    loop {
        let action = napcat.action().await;
        if action["action"] == "send_private_msg" {
            let text = &action["params"]["message"][0]["data"]["text"];
            return text.as_str().unwrap_or_default().to_string();
        }
    }
}

/// 下一个发进群里的消息的段，和在它以前 NapCat 收到的别的动作的名字。
async fn next_group_message(napcat: &mut Answering) -> (Vec<Value>, Vec<String>) {
    let mut before = Vec::new();
    loop {
        let action = napcat.action().await;
        if action["action"] == "send_group_msg" {
            let message = action["params"]["message"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            return (message, before);
        }
        before.push(action["action"].as_str().unwrap_or_default().to_string());
    }
}

#[tokio::test]
async fn a_poke_in_private_opens_a_turn() {
    let lines = Lines::new([Line::says("在"), Line::says("在呢")]);
    let (home, mut napcat) = up(&lines).await;
    // 先说一句：会话标题设成他的名字。
    napcat.send(private_frame(ADMIN, 1, json!([plain("在吗")])));
    assert_eq!(next_private(&mut napcat).await, "在");
    napcat.send(poke(ADMIN, BOT, None, nudge()));
    assert_eq!(next_private(&mut napcat).await, "在呢");
    let mine = home.own_sessions();
    let events = home.events(&mine[0]);
    let heard = said(&events);
    assert_eq!(heard.len(), 2, "{events:?}");
    let venue = &heard[1]["body"]["venue"];
    assert_eq!(venue["poke"], json!(true), "{venue}");
    assert_eq!(venue["poke_text"], json!("的小脑袋"), "{venue}");
    assert!(
        heard[1]["body"]["blocks"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "没有字"
    );
    // 戳一戳没带名字：标题照旧是他的名字，不拿号盖掉。
    let titles: Vec<_> = of_kind(&events, "session.meta_changed")
        .iter()
        .map(|event| event["body"]["title"].clone())
        .collect();
    assert_eq!(titles, [json!("someone")], "{titles:?}");
    assert!(
        run_log(&home.root).contains("poke raw"),
        "整份 raw_info 记一行"
    );
    // 核心照 `poke` 那一格画成平台事实（VP-1），后缀接在后面。
    let text = user_text(&lines.requests()[1]);
    assert!(
        text.contains("poked you") && text.contains("的小脑袋"),
        "{text}"
    );
    stopped(home).await;
}

#[tokio::test]
async fn a_poke_in_a_group_is_judged_like_a_call_and_poking_others_is_only_recorded() {
    let lines = Lines::new([Line::says("嗯？")]);
    let (home, mut napcat) = up(&lines).await;
    // 群友戳别人：只记进聊天记录。
    napcat.send(poke(MEMBER, ADMIN, Some(GROUP), json!([])));
    // 群友戳她：照叫她办，开一轮。
    napcat.send(poke(MEMBER, BOT, Some(GROUP), json!([])));
    let (sent, before) = next_group_message(&mut napcat).await;
    assert!(
        sent.iter().all(|segment| segment["type"] != "reply"),
        "那一条平台上没有，不引用：{sent:?}"
    );
    assert_eq!(sent.last().expect("有字")["data"]["text"], "嗯？");
    let events = venue_events(&home.root, &venue());
    let heard = said(&events);
    assert_eq!(heard.len(), 1, "戳别人的不进会话：{heard:?}");
    assert_eq!(heard[0]["body"]["venue"]["poke"], json!(true));
    assert!(
        heard[0]["body"]["venue"].get("mentions_me").is_none(),
        "不写 @ 了她：{}",
        heard[0]
    );
    assert!(
        !before.iter().any(|action| action == "set_msg_emoji_like"),
        "不贴表情：{before:?}"
    );
    stopped(home).await;
}
