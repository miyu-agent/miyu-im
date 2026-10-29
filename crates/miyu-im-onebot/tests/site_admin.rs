//! 群管理在 im 页面上（施工 Q-6）：真核心拉起真桥、假 NapCat，页面经通道要群管理的内容。驱动带的插件不经核心的 `package.call`，
//! 在进程里直接问：会话浮窗的「群管理」页（刷屏处理的表单、记录），存刷屏处理写进按群的设置，面板一个群一项、带她的身份。

use std::sync::Arc;

use serde_json::{Value, json};

use crate::support::browser::{call, logged_in};
use crate::support::group::*;
use crate::support::head::Head;
use crate::support::speaking::Lines;
use crate::support::*;

/// 假 NapCat 认得的群成员：她自己，是管理员。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 群管理的编号。
const ADMIN: &str = "miyu-im-qq-admin";

/// 要群管理的一块内容。
fn asking(place: &str, more: Value) -> Value {
    let mut params = json!({"package": ADMIN, "place": place, "language": "zh"});
    params
        .as_object_mut()
        .expect("是对象")
        .extend(more.as_object().cloned().unwrap_or_default());
    params
}

#[tokio::test(flavor = "multi_thread")]
async fn the_group_admin_panel_and_page_come_from_the_driver() {
    let (home, _napcat, _) = started_ranked(
        Arc::new(Lines::new([])),
        "",
        "",
        (MEMBERS, &[(BOT, "admin")]),
    )
    .await;
    let mut head = Head::connect(&home.root).await;
    let status = head.until("status", |status| status["web"].is_u64()).await;
    let web = u16::try_from(status["web"].as_u64().expect("有端口")).expect("是端口");
    let mut open = logged_in(&home.root, web).await;

    let venue = json!({"venue": {"type": "group", "id": "777"}});
    let panel = call(
        &mut open,
        1,
        "plugins.widget",
        asking("panel", venue.clone()),
    )
    .await;
    let content = &panel["result"]["content"];
    assert_eq!(
        content[0],
        json!({"type": "heading", "text": "刷屏处理"}),
        "{panel}"
    );
    assert_eq!(content[1]["fields"][0]["value"], false, "出厂关着：{panel}");

    let save = json!({"package": ADMIN, "method": "admin.spam", "params": {"group": 777, "values": {"on": true, "mute": 5}}});
    let saved = call(&mut open, 2, "plugins.call", save).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let groups: Value = serde_json::from_str(
        &std::fs::read_to_string(
            home.root
                .state()
                .join("packages/miyu-im/qq/admin/groups.json"),
        )
        .expect("写了"),
    )
    .expect("是 JSON");
    assert_eq!(groups["777"]["spam"]["on"], true, "{groups}");
    assert_eq!(groups["777"]["spam"]["mute"], 300, "{groups}");
    let panel = call(&mut open, 3, "plugins.widget", asking("panel", venue)).await;
    assert_eq!(
        panel["result"]["content"][1]["fields"][0]["value"], true,
        "{panel}"
    );

    let page = call(
        &mut open,
        4,
        "plugins.widget",
        asking("page", json!({"page": "admin"})),
    )
    .await;
    let items = &page["result"]["content"][0]["items"];
    let shown: Vec<(Value, Value)> = items
        .as_array()
        .expect("一个群一项")
        .iter()
        .map(|item| (item["subtitle"].clone(), item["badge"]["text"].clone()))
        .collect();
    assert_eq!(
        shown,
        [
            (json!("777"), json!("管理员")),
            (json!("666"), json!("管理员"))
        ],
        "{page}"
    );
    stopped(home).await;
}
