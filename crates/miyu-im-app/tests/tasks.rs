//! 会话的定时任务（施工 I-198，设计 12 第一节第 3 条）：真核心照开关拉起真桥、假 NapCat，经网页的样子调 `tasks.*`。给名单里的群卡建
//! 一个几秒以后的任务：列得出；到点她在这个群开一轮，请求里带 `<scheduled-task>` 包着的那段，说的话发到群里；卡片「是否回复」
//! 关着的不做；删了的不再响；别的卡看不到、删不了这张卡的任务。

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use miyu_kernel::time::Timestamp;

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 名单里的群。
const GROUP: i64 = 779;

/// 另一个名单里的群。
const OTHER: i64 = 780;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 系统的场所规则：只有一条配群的空规则。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\n";

/// 此刻以后 `after` 那一刻，写成核心认的样子。
fn at(after: Duration) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("在 1970 年以后");
    let millis = i64::try_from((now + after).as_millis()).expect("放得下");
    Timestamp::from_unix_millis(millis)
        .expect("在范围里")
        .to_string()
}

/// 群 `group` 的卡。
fn card(group: i64) -> Value {
    json!({"kind": "group", "id": group})
}

/// 调页面的方法 `method`，交回答的。
async fn call(head: &mut Head, method: &str, params: Value) -> Value {
    let answered = head.page_call_with(method, params).await;
    assert!(answered["error"].is_null(), "{method}：{answered}");
    answered["result"].clone()
}

#[tokio::test]
async fn a_task_on_a_group_card_opens_a_turn_there_when_due() {
    let lines = Lines::new([Line::says("今天晴")]);
    let (home, mut napcat, _) = started_by(Arc::new(lines.clone()), RULES, MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let cards = json!({"groups": [{"id": GROUP, "set": {}}, {"id": OTHER, "set": {"on": false}}]});
    let saved = call(&mut head, "cards.save", json!({"cards": cards})).await;
    assert_eq!(saved, json!({"saved": true}));

    // 时间写得不对的：核心拒的那一句交回来，不建。
    let bad = json!({"card": card(GROUP), "text": "报天气", "when": {"cron": "不是时间"}});
    let refused = call(&mut head, "tasks.save", bad).await;
    assert_eq!(refused["saved"], false, "{refused}");
    assert!(refused["message"].is_string(), "{refused}");

    // 回复关着的群：到点不做。
    let off = json!({"card": card(OTHER), "text": "不该做的", "when": {"at": at(Duration::from_secs(2))}});
    assert_eq!(call(&mut head, "tasks.save", off).await["saved"], true);
    // 删了的不再响。
    let gone =
        json!({"card": card(GROUP), "text": "删掉的", "when": {"at": at(Duration::from_secs(2))}});
    let gone = call(&mut head, "tasks.save", gone).await["id"].clone();
    let task = json!({"card": card(GROUP), "text": "报一下\"天气\"", "when": {"at": at(Duration::from_secs(3))}});
    let made = call(&mut head, "tasks.save", task).await;
    assert_eq!(made["saved"], true, "{made}");

    let listed = call(&mut head, "tasks.list", json!({"card": card(GROUP)})).await;
    let tasks = listed["tasks"].as_array().expect("是列表");
    assert_eq!(tasks.len(), 2, "只列这张卡的：{listed}");
    assert_eq!(tasks[1]["text"], "报一下\"天气\"");
    assert_eq!(tasks[1]["id"], made["id"]);
    assert!(tasks[1]["next"].is_string(), "{listed}");
    // 别的卡删不了这张卡的任务。
    let stolen = head
        .page_call_with("tasks.remove", json!({"card": card(OTHER), "id": gone}))
        .await;
    assert!(!stolen["error"].is_null(), "{stolen}");
    let removed = call(
        &mut head,
        "tasks.remove",
        json!({"card": card(GROUP), "id": gone}),
    )
    .await;
    assert_eq!(removed, json!({"removed": true}));

    assert_eq!(napcat.group_reply(GROUP).await, "今天晴");
    let requests = lines.requests();
    assert_eq!(requests.len(), 1, "只有那一个任务请求了模型：{requests:#?}");
    let last = serde_json::to_string(requests[0].messages.last().expect("有消息")).expect("写得成");
    assert!(last.contains("<scheduled-task>"), "{last}");
    assert!(
        last.contains("报一下\\\\u0022天气\\\\u0022</scheduled-task>"),
        "人写的那段照模板转义：{last}"
    );
    assert!(napcat.pending().is_none(), "回复关着的群、删掉的都没说话");
    let after = call(&mut head, "tasks.list", json!({"card": card(GROUP)})).await;
    assert_eq!(after["tasks"], json!([]), "一次的响过就没了：{after}");
    stopped(home).await;
}
