//! 同步群里别的管理员做的（施工 Q-3，设计 02 第一节第 2、3 条）：真核心拉起真桥、假 NapCat 推通知。群管（`MANAGER`）禁言、解禁、
//! 踢人：记进管理记录（谁做的是他，带名字），给这个群的会话记一句 `moderated`，不开回合；她做的、对她做的不记；「同步解禁」关了的
//! 解禁不记；她不是管理员的群不管。

use std::sync::Arc;

use serde_json::{Value, json};

use crate::support::group::*;
use crate::support::platform::*;
use crate::support::speaking::Lines;
use crate::support::*;

/// 一帧通知：`notice_type`、`sub_type`，对 `user`、`operator` 做的，禁 `duration` 秒。
fn notice(kind: &str, sub: &str, (user, operator): (i64, i64), duration: Option<u64>) -> Value {
    let mut frame = json!({"post_type": "notice", "notice_type": kind, "sub_type": sub, "time": TIME, "self_id": BOT, "group_id": GROUP, "user_id": user, "operator_id": operator});
    if let Some(duration) = duration {
        frame["duration"] = json!(duration);
    }
    frame
}

/// 这个群的管理记录。
fn records(home: &Home) -> Vec<Value> {
    let path = home
        .root
        .state()
        .join("packages/miyu-im/qq/admin")
        .join(format!("{GROUP}.jsonl"));
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).expect("一行一条 JSON"))
        .collect()
}

/// 等到这个群的会话记了 `count` 句 `moderated`：交回那几句的字。
async fn noted(home: &Home, count: usize) -> Vec<String> {
    let events = until_events(&home.root, &venue(), |events| {
        moderated(events).len() >= count
    })
    .await;
    moderated(&events)
}

/// 记下的 `moderated` 的字，照先后。
fn moderated(events: &[Value]) -> Vec<String> {
    of_kind(events, "context.injected")
        .iter()
        .filter(|event| event["body"]["kind"] == "moderated")
        .filter_map(|event| event["body"]["text"].as_str().map(str::to_string))
        .collect()
}

/// 出厂的模板换出来、套上插件记事实的外壳的那一句。
fn fact(name: &str, fields: &[(&str, &str)]) -> String {
    let mut text = std::fs::read_to_string(
        resources().join(format!("packages/miyu-im/data/onebot/facts/{name}.txt")),
    )
    .expect("读得出");
    for (field, value) in fields {
        text = text.replace(&format!("{{{field}}}"), value);
    }
    format!("<moderated>{}</moderated>\n", text.trim())
}

#[tokio::test]
async fn other_admins_mutes_unmutes_and_kicks_are_recorded_and_told_to_her() {
    let lines = Lines::new([]);
    let (home, napcat) = up(&lines).await;
    napcat.send(notice("group_ban", "ban", (MEMBER, MANAGER), Some(600)));
    napcat.send(notice("group_ban", "lift_ban", (MEMBER, MANAGER), Some(0)));
    napcat.send(notice("group_decrease", "kick", (IMPOSTOR, MANAGER), None));
    // 她做的、对她做的：不记。
    napcat.send(notice("group_ban", "ban", (MEMBER, BOT), Some(60)));
    let said = noted(&home, 3).await;
    assert_eq!(
        said,
        [
            fact(
                "muted-by",
                &[("who", "小明"), ("by", "群管"), ("duration", "10 minutes")]
            ),
            fact("unmuted-by", &[("who", "小明"), ("by", "群管")]),
            fact("kicked-by", &[("who", "终端管理员"), ("by", "群管")]),
        ]
    );
    let written = within("记下三条", async {
        loop {
            let written = records(&home);
            if written.len() >= 3 {
                return written;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    let by = json!({"kind": "admin", "user": MANAGER, "name": "群管"});
    let shape: Vec<(Value, Value, Value)> = written
        .iter()
        .map(|record| {
            (
                record["action"].clone(),
                record["user"].clone(),
                record["by"].clone(),
            )
        })
        .collect();
    assert_eq!(
        shape,
        [
            (json!("mute"), json!(MEMBER), by.clone()),
            (json!("unmute"), json!(MEMBER), by.clone()),
            (json!("kick"), json!(IMPOSTOR), by),
        ]
    );
    assert_eq!(written[0]["seconds"], 600);
    // 不开回合。
    let events = until_events(&home.root, &venue(), |_| true).await;
    assert!(of_kind(&events, "turn.started").is_empty());
    assert_eq!(records(&home).len(), 3, "她做的不记");
    stopped(home).await;
}

#[tokio::test]
async fn unmutes_follow_their_switch_and_groups_where_she_is_no_admin_are_left_alone() {
    let lines = Lines::new([]);
    let config = format!("{}qq.admin_sync_unmute = false\n", onebot());
    let (home, napcat, _) =
        started_ranked(Arc::new(lines.clone()), &rules(), &config, (MEMBERS, RANKS)).await;
    napcat.send(notice("group_ban", "lift_ban", (MEMBER, MANAGER), Some(0)));
    napcat.send(notice("group_ban", "ban", (MEMBER, MANAGER), Some(60)));
    assert_eq!(
        noted(&home, 1).await,
        [fact(
            "muted-by",
            &[("who", "小明"), ("by", "群管"), ("duration", "1 minute")]
        )],
        "同步解禁关了，解禁不记"
    );
    stopped(home).await;
    // 她是普通成员的群：不管。
    let ranks = [(OWNER, "owner")];
    let (home, napcat, _) = started_ranked(
        Arc::new(lines.clone()),
        &rules(),
        &onebot(),
        (MEMBERS, &ranks),
    )
    .await;
    napcat.send(notice("group_ban", "ban", (MEMBER, MANAGER), Some(60)));
    within("问过她的身份", async {
        while !napcat.asked().contains(&BOT) {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(records(&home).is_empty(), "不是管理员的群不记");
    let events = until_events(&home.root, &venue(), |_| true).await;
    assert!(moderated(&events).is_empty(), "也不告诉她");
    stopped(home).await;
}

#[tokio::test]
async fn a_group_with_group_admin_switched_off_is_left_alone() {
    let lines = Lines::new([]);
    let (home, napcat) = up(&lines).await;
    // 在这个群关掉群管理（「陌生群」卡上的插件开关）。
    let mut head = crate::support::head::Head::connect(&home.root).await;
    let off = json!({"card": {"kind": "other_groups"}, "values": {"plugins": {"miyu-im-qq-admin": false}}});
    let saved = head.page_call_with("cards.save_one", off).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    napcat.send(notice("group_ban", "ban", (MEMBER, MANAGER), Some(60)));
    // 通知照样交给挂接点：等桥记下它读到了，再看没记。
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(records(&home).is_empty(), "关着的群不记");
    let events = until_events(&home.root, &venue(), |_| true).await;
    assert!(moderated(&events).is_empty(), "也不告诉她");
    assert!(!napcat.asked().contains(&BOT), "不问她的身份");
    stopped(home).await;
}
