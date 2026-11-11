//! 卡片的多模态模型（施工 I-224，todolist 第 34 条，核心 VM-1）：真核心照开关拉起真桥，系统配置多一家 `judge`（判官的假服务器）。
//! 卡片的 `rules.vision` 设了的群，会话造好、交第一条消息以前就钉上（核心记 `session.policy_changed` 的 `vision`）；换了的跟着换，
//! 清掉的回到跟着配置的 `models.vision`（桥交 `vision: null`，核心记空的）。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::judge::judge;
use crate::support::*;

/// 群号。
const GROUP: i64 = 3941;

/// 群里的人。
const LIN: i64 = 20004;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 系统的场所规则：只有一条配群的空规则。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\n";

/// 会话日志里钉看图模型的几条：照先后（去掉的核心记空的）。
fn pinned(events: &[Value]) -> Vec<Value> {
    events
        .iter()
        .filter(|event| {
            event["kind"] == "session.policy_changed" && event["body"].get("vision").is_some()
        })
        .map(|event| event["body"]["vision"].clone())
        .collect()
}

/// 存卡片：这个群的 `rules` 写 `rules`。
async fn save(head: &mut Head, rules: Value) {
    let cards = json!({"groups": [{"id": GROUP, "set": {"rules": rules}}]});
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
}

#[tokio::test]
async fn a_card_pins_the_vision_model_and_follows_its_changes() {
    let server = judge(vec![]).await;
    let (home, napcat, _) =
        started_judged(&Script::new([]), &server, (RULES, ""), "", MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    save(&mut head, json!({"vision": "judge/eyes"})).await;
    napcat.send(group_frame(
        GROUP,
        LIN,
        1,
        json!([plain("大家好")]),
        ("小林", "lin"),
    ));
    let venue = format!("qq:30003:group:{GROUP}");
    let events = until_events(&home.root, &venue, |events| {
        events.iter().any(|event| event["kind"] == "message.user")
    })
    .await;
    assert_eq!(
        pinned(&events),
        [json!("judge/eyes")],
        "交第一条消息以前钉上"
    );
    save(&mut head, json!({"vision": "judge/glasses"})).await;
    let events = until_events(&home.root, &venue, |events| pinned(events).len() >= 2).await;
    assert_eq!(pinned(&events)[1], json!("judge/glasses"));
    // 清掉：回到跟着 `models.vision`。
    save(&mut head, json!({})).await;
    let events = until_events(&home.root, &venue, |events| pinned(events).len() >= 3).await;
    let cleared = &pinned(&events)[2];
    assert!(
        cleared.is_null() || cleared == "",
        "去掉的记空的：{cleared}"
    );
    stopped(home).await;
}
