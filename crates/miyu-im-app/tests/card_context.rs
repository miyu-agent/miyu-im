//! 卡片的上下文上限（施工 I-189，核心 CTX-1）：真核心照开关拉起真桥。卡片的 `rules.context` 设了的群，会话造好、交第一条消息以前
//! 就单独设上（核心记 `session.policy_changed` 的 `context`）；设得太小的核心不收，会话页问卡片时 `context_min` 交这个群最少要多少；
//! 改成够的、清掉的跟着设（清掉的交 `null`），`context_min` 拿掉。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::judge::judge;
use crate::support::*;

/// 群号。
const GROUP: i64 = 3931;

/// 群里的人。
const LIN: i64 = 20004;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 系统的场所规则：只有一条配群的空规则。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\n";

/// 会话日志里换上下文上限的几条：照先后的 token 数（核心记 `{"tokens"}`，去掉的是 `null`）。
fn limits(events: &[Value]) -> Vec<Value> {
    events
        .iter()
        .filter(|event| {
            event["kind"] == "session.policy_changed" && event["body"].get("context").is_some()
        })
        .map(|event| match &event["body"]["context"] {
            Value::Object(context) => context.get("tokens").cloned().unwrap_or(Value::Null),
            other => other.clone(),
        })
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

/// 问卡片：交回 `context_min`。
async fn context_min(head: &mut Head) -> Value {
    let got = head
        .page_call_with("cards.get", json!({"platform": "qq"}))
        .await;
    got["result"]["context_min"].clone()
}

#[tokio::test]
async fn a_card_sets_the_context_limit_and_says_when_it_is_too_small() {
    let server = judge(vec![]).await;
    let (home, napcat, _) =
        started_judged(&Script::new([]), &server, (RULES, ""), "", MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    save(&mut head, json!({"context": 128_000})).await;
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
    assert_eq!(limits(&events), [json!(128_000)], "交第一条消息以前设上");
    assert_eq!(
        context_min(&mut head).await,
        json!({"group": {}, "private": {}})
    );
    // 太小：核心不收、什么都不换，问卡片时说这个群最少要多少。
    save(&mut head, json!({"context": 10})).await;
    let deadline = tokio::time::Instant::now() + WAIT;
    let min = loop {
        let found = context_min(&mut head).await;
        if let Some(min) = found["group"][GROUP.to_string()].as_u64() {
            break min;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "等不到最少多少：{found}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    };
    assert!(min > 10, "{min}");
    // 清掉：交 `null`，回到照模型的窗口，最少多少拿掉。
    save(&mut head, json!({})).await;
    let events = until_events(&home.root, &venue, |events| limits(events).len() >= 2).await;
    assert_eq!(limits(&events), [json!(128_000), Value::Null]);
    let deadline = tokio::time::Instant::now() + WAIT;
    while context_min(&mut head).await["group"]
        .get(GROUP.to_string())
        .is_some()
    {
        assert!(tokio::time::Instant::now() < deadline, "最少多少没拿掉");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    stopped(home).await;
}
