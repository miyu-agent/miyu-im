//! 读群的场所会话的事件、桥的运行日志（施工 O-58 上从 `group.rs` 挪来：那个文件过了 500 行）；`group.rs` 照旧再导出。

use std::time::Duration;

use serde_json::Value;

use miyu_kernel::id::AccountId;
use miyu_store::log::read_events;
use miyu_store::root::DataRoot;

use super::group::WAIT;

/// 系统账号 `onebot` 名下、场所是 `venue` 的那个会话（主线）：编号和事件（写成 JSON）；还没有的是空的。分叉出来的支线（施工
/// O-36：`session.created` 带 `forked`，场所照主线的）不算。
pub fn venue_session(root: &DataRoot, venue: &str) -> Option<(String, Vec<Value>)> {
    venue_sessions(root, venue)
        .into_iter()
        .find(|(_, events)| events[0]["body"]["forked"].is_null())
}

/// 系统账号 `onebot` 名下、场所是 `venue` 的分叉出来的支线（施工 O-36），照分叉的先后：编号和事件。
pub fn lane_sessions(root: &DataRoot, venue: &str) -> Vec<(String, Vec<Value>)> {
    let mut lanes: Vec<_> = venue_sessions(root, venue)
        .into_iter()
        .filter(|(_, events)| !events[0]["body"]["forked"].is_null())
        .collect();
    lanes.sort_by_key(|(_, events)| events[0]["at"].as_str().unwrap_or_default().to_string());
    lanes
}

/// 系统账号 `onebot` 名下、场所是 `venue` 的会话（主线、支线都算）：编号和事件。
fn venue_sessions(root: &DataRoot, venue: &str) -> Vec<(String, Vec<Value>)> {
    let system = AccountId::parse("onebot").expect("合写法");
    root.sessions(&system)
        .unwrap_or_default()
        .iter()
        .map(|session| {
            let events: Vec<Value> = read_events(&root.session_dir(&system, session))
                .unwrap_or_default()
                .iter()
                .map(|event| serde_json::to_value(event).expect("写得成 JSON"))
                .collect();
            (session.as_str().to_string(), events)
        })
        .filter(|(_, events)| {
            events
                .first()
                .is_some_and(|first| first["body"]["venue"] == venue)
        })
        .collect()
}

/// 场所是 `venue` 的那个会话的事件；还没有的是空的。
pub fn venue_events(root: &DataRoot, venue: &str) -> Vec<Value> {
    venue_session(root, venue)
        .map(|(_, events)| events)
        .unwrap_or_default()
}

/// 等到场所 `venue` 的会话里有合 `wanted` 的事件：交回那时的全部事件。
pub async fn until_event(
    root: &DataRoot,
    venue: &str,
    wanted: impl Fn(&Value) -> bool,
) -> Vec<Value> {
    until_events(root, venue, |events| events.iter().any(&wanted)).await
}

/// 等到场所 `venue` 的会话的事件合 `wanted`：交回那时的全部事件。
pub async fn until_events(
    root: &DataRoot,
    venue: &str,
    wanted: impl Fn(&[Value]) -> bool,
) -> Vec<Value> {
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        let events = venue_events(root, venue);
        if wanted(&events) {
            return events;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "{venue} 等不到：{events:#?}\n运行日志：\n{}",
            run_log(root)
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// 事件里种类是 `kind` 的，照先后。
pub fn of_kind(events: &[Value], kind: &str) -> Vec<Value> {
    events
        .iter()
        .filter(|event| event["kind"] == kind)
        .cloned()
        .collect()
}

/// 事件里人说的话（`message.user`），照先后。
pub fn said(events: &[Value]) -> Vec<Value> {
    of_kind(events, "message.user")
}

/// 一条人说的话的字。
pub fn words(said: &Value) -> &str {
    said["body"]["blocks"][0]["text"]
        .as_str()
        .unwrap_or_default()
}

/// 运行日志。
pub fn run_log(root: &DataRoot) -> String {
    std::fs::read_to_string(root.state().join("logs").join("onebot.log")).unwrap_or_default()
}

/// 等到运行日志合 `wanted`，交回那一刻的全文：桥常常先做了事（NapCat 收到了动作）、等回应以后才写日志，测试收到动作就读会
/// 早一步。最多等六十秒。
pub async fn until_logged(root: &DataRoot, wanted: impl Fn(&str) -> bool) -> String {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        let log = run_log(root);
        if wanted(&log) {
            return log;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "运行日志等不到：{log}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}
