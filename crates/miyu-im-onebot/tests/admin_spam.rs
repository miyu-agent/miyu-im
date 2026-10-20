//! 刷屏处理（施工 Q-5，设计 02 第一节第 7 条）：真核心拉起真桥、假 NapCat。这个群开了刷屏处理（`groups.json`，出厂关着）：小明自己连着
//! 发三条一样的话（别人插话不打断），后两条撤回、禁言这个群设的时长，记进管理记录（谁做的是刷屏处理），给会话记一句 `moderated`；
//! 第三条不交给 im。没开的群照常。

use serde_json::{Value, json};

use crate::support::group::*;
use crate::support::platform::*;
use crate::support::speaking::Lines;
use crate::support::*;

/// 驱动的状态目录。
fn store(home: &Home) -> std::path::PathBuf {
    home.root.state().join("packages/im/qq")
}

/// 写这个群的刷屏处理：开着，禁 `mute` 秒。
fn spam_on(home: &Home, mute: u64) {
    let dir = store(home).join("admin");
    std::fs::create_dir_all(&dir).expect("建得了");
    let groups = json!({GROUP.to_string(): {"spam": {"on": true, "mute": mute}}});
    std::fs::write(dir.join("groups.json"), groups.to_string()).expect("写得进");
}

/// `user` 发第 `message` 条 `text`，普通成员。
fn posts(napcat: &Answering, user: i64, message: i64, text: &str) {
    napcat.send(group_frame_as(
        GROUP,
        user,
        message,
        json!([plain(text)]),
        names(user),
        "member",
    ));
}

/// 这个群的会话里记下的人说的话的编号。
fn heard(events: &[Value]) -> Vec<String> {
    of_kind(events, "message.user")
        .iter()
        .filter_map(|event| event["body"]["venue"]["msg"].as_str().map(str::to_string))
        .collect()
}

#[tokio::test]
async fn a_flood_is_recalled_muted_recorded_and_told_but_not_passed_on() {
    let (home, mut napcat) = up(&Lines::new([])).await;
    spam_on(&home, 120);
    posts(&napcat, MEMBER, 1101, "刷屏啦");
    // 开了刷屏处理的群一有人说话先问她的身份：问到了，第三条才照它定交不交给 im。
    within("问她的身份", async {
        while !napcat.asked().contains(&BOT) {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    posts(&napcat, FRIEND, 1102, "别刷了");
    posts(&napcat, MEMBER, 1103, "刷屏啦");
    posts(&napcat, MEMBER, 1104, " 刷屏啦 ");
    posts(&napcat, FRIEND, 1105, "好了");
    assert_eq!(napcat.recalled().await, json!({"message_id": 1103}));
    assert_eq!(napcat.recalled().await, json!({"message_id": 1104}));
    let muted = next_platform(&mut napcat).await;
    // 第四条：只撤回，不再禁一次。
    posts(&napcat, MEMBER, 1106, "刷屏啦");
    assert_eq!(napcat.recalled().await, json!({"message_id": 1106}));
    assert_eq!(muted["action"], "set_group_ban", "{muted}");
    assert_eq!(
        muted["params"],
        json!({"group_id": GROUP, "user_id": MEMBER, "duration": 120})
    );
    let events = until_events(&home.root, &venue(), |events| {
        of_kind(events, "context.injected")
            .iter()
            .any(|event| event["body"]["kind"] == "moderated")
            && heard(events).contains(&"1105".to_string())
    })
    .await;
    let said =
        std::fs::read_to_string(resources().join("packages/im/data/onebot/facts/spam-muted.txt"))
            .expect("读得出")
            .replace("{who}", "小明")
            .replace("{duration}", "2 minutes");
    let noted: Vec<Value> = of_kind(&events, "context.injected")
        .into_iter()
        .filter(|event| event["body"]["kind"] == "moderated")
        .map(|event| event["body"]["text"].clone())
        .collect();
    assert_eq!(
        noted,
        [json!(format!("<moderated>{}</moderated>\n", said.trim()))]
    );
    assert_eq!(
        heard(&events),
        ["1101", "1102", "1103", "1105"],
        "第三条不交给 im"
    );
    let record: Value = within("记下了", async {
        loop {
            let text = std::fs::read_to_string(store(&home).join(format!("admin/{GROUP}.jsonl")))
                .unwrap_or_default();
            if let Some(line) = text.lines().next() {
                return serde_json::from_str(line).expect("是 JSON");
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    assert_eq!(record["by"], json!({"kind": "spam"}), "{record}");
    assert_eq!(record["seconds"], 120, "{record}");
    // 第四条只撤回：等它那一趟办完，没再禁言、没再记。
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    no_platform(&mut napcat);
    let lines = std::fs::read_to_string(store(&home).join(format!("admin/{GROUP}.jsonl")))
        .unwrap_or_default();
    assert_eq!(lines.lines().count(), 1, "{lines}");
    stopped(home).await;
}

#[tokio::test]
async fn a_group_without_spam_handling_is_left_alone() {
    let (home, mut napcat) = up(&Lines::new([])).await;
    for message in 1201..=1204 {
        posts(&napcat, MEMBER, message, "刷屏啦");
    }
    let events = until_events(&home.root, &venue(), |events| heard(events).len() >= 4).await;
    assert_eq!(heard(&events), ["1201", "1202", "1203", "1204"], "出厂关着");
    no_platform(&mut napcat);
    stopped(home).await;
}

/// `user` 以身份 `role` 连发 `messages` 那几条同样的话。
fn floods(napcat: &Answering, user: i64, role: &str, messages: std::ops::RangeInclusive<i64>) {
    for message in messages {
        napcat.send(group_frame_as(
            GROUP,
            user,
            message,
            json!([plain("刷屏啦")]),
            names(user),
            role,
        ));
    }
}

#[tokio::test]
async fn admins_floods_and_groups_with_group_admin_off_are_left_alone() {
    let (home, mut napcat) = up(&Lines::new([])).await;
    spam_on(&home, 60);
    // 群管理员刷屏：禁不了，不管。
    floods(&napcat, QQ_ADMIN, "admin", 1301..=1303);
    let events = until_events(&home.root, &venue(), |events| heard(events).len() >= 3).await;
    assert_eq!(heard(&events), ["1301", "1302", "1303"]);
    // 这个群关了群管理（「陌生群」卡上的插件开关）：不管。
    let mut head = crate::support::head::Head::connect(&home.root).await;
    let off = json!({"card": {"kind": "other_groups"}, "values": {"plugins": {"miyu-im-qq-admin": false}}});
    let saved = head.page_call_with("cards.save_one", off).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    floods(&napcat, MEMBER, "member", 1311..=1313);
    let events = until_events(&home.root, &venue(), |events| heard(events).len() >= 6).await;
    assert_eq!(heard(&events)[3..], ["1311", "1312", "1313"]);
    no_platform(&mut napcat);
    stopped(home).await;
}

#[tokio::test]
async fn a_flood_where_she_is_no_admin_is_passed_on() {
    let ranks = [(OWNER, "owner")];
    let (home, mut napcat, _) = crate::support::group::started_ranked(
        std::sync::Arc::new(Lines::new([])),
        &rules(),
        &onebot(),
        (MEMBERS, &ranks),
    )
    .await;
    spam_on(&home, 60);
    floods(&napcat, MEMBER, "member", 1401..=1404);
    let events = until_events(&home.root, &venue(), |events| heard(events).len() >= 4).await;
    assert_eq!(heard(&events), ["1401", "1402", "1403", "1404"]);
    assert!(napcat.asked().contains(&BOT), "问过她的身份");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    no_platform(&mut napcat);
    stopped(home).await;
}
