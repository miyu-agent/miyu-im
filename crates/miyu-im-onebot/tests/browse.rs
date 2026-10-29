//! 后台「消息记录」页的场所会话（施工 I-34 上）：真核心拉起真桥、假 NapCat，人的头经核心的 `package.call` 调 im 的两个方法。会话照
//! 收进它的机器人号分组、带群名；删得掉自己的，不认得的会话拒 `session_not_found`。

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::answering::GROUP_NAME;
use crate::support::group::*;
use crate::support::head::Head;
use crate::support::*;

/// 群号：假 NapCat 的群列表里有它（`LISTED_GROUPS`），带群名。
const GROUP: i64 = 777;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 系统的场所规则：群都不抽样。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0 }\n";

/// 回答里的 `result`，拒了的照原样交回整条好看清。
fn result(reply: &Value) -> &Value {
    assert!(reply.get("result").is_some(), "{reply}");
    &reply["result"]
}

#[tokio::test]
async fn the_page_lists_and_deletes_her_sessions() {
    let script = Script::new([Play::Says("在呢")]);
    let (home, mut napcat, _) = started(&script, RULES, "", MEMBERS).await;
    let venue = format!("qq:30003:group:{GROUP}");
    napcat.send(group_frame(
        GROUP,
        ADMIN,
        1,
        json!([at(BOT), plain(" 在吗")]),
        ("终端管理员", "o"),
    ));
    assert_eq!(napcat.group_reply(GROUP).await, "在呢");
    until_events(&home.root, &venue, |events| {
        !of_kind(events, "turn.ended").is_empty()
    })
    .await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    // 列：照收进它的机器人号分组，带群名。
    let listed = head.page_call("sessions.list").await;
    let connections = result(&listed)["connections"].clone();
    assert_eq!(connections[0]["bot"], BOT, "{connections}");
    let one = &connections[0]["sessions"][0];
    assert_eq!(
        (&one["venue"], &one["kind"], &one["id"], &one["name"]),
        (
            &json!(venue),
            &json!("group"),
            &json!(GROUP),
            &json!(GROUP_NAME)
        ),
        "{one}"
    );
    let session = one["session"].as_str().expect("有会话").to_string();
    // 不认得的会话：拒。
    let stranger = head
        .page_call_with(
            "sessions.delete",
            json!({"session": "0190f3a1-0000-7000-8000-000000000009"}),
        )
        .await;
    assert_eq!(
        stranger["error"]["data"]["reason"], "method_failed",
        "{stranger}"
    );
    // 删：删掉以后列不出来。
    let deleted = head
        .page_call_with("sessions.delete", json!({"session": session}))
        .await;
    assert_eq!(result(&deleted), &json!({"deleted": true}));
    let listed = head.page_call("sessions.list").await;
    let left: Vec<&Value> = result(&listed)["connections"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|group| group["sessions"].as_array().into_iter().flatten())
        .filter(|one| one["session"] == session.as_str())
        .collect();
    assert!(left.is_empty(), "{listed}");
    stopped(home).await;
}
