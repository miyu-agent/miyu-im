//! 实时日志的汇总（施工 I-160 中）。

use std::sync::Arc;

use serde_json::{Value, json};

use super::{Hub, KEEP};
use crate::watch::{Block, Item};

fn heard(seq: u64, at: &str, who: &str, text: &str) -> Value {
    json!({"seq": seq, "at": at, "kind": "message.user", "body": {
        "blocks": [{"type": "text", "text": text}], "venue": {"name": who, "place": "Arch 群"}}})
}

fn kinds(items: &[Item]) -> Vec<&'static str> {
    items
        .iter()
        .map(|item| match item {
            Item::Heard { .. } => "heard",
            Item::Started { .. } => "started",
            Item::Wrote { done: true, .. } => "wrote",
            Item::Wrote { done: false, .. } => "writing",
            Item::Appended { .. } => "appended",
            _ => "other",
        })
        .collect()
}

fn kept(hub: &Hub, venue: &str) -> Vec<Item> {
    hub.recent(Some(venue))
        .into_iter()
        .flat_map(|batch| batch.lines)
        .map(|line| line.item)
        .collect()
}

#[test]
fn settled_lines_are_kept_and_names_found_later() {
    let hub = Hub::default();
    hub.heard(
        "g",
        "main",
        &heard(7, "2026-11-05T11:00:00.000Z", "小林", "在吗"),
    );
    hub.heard(
        "g",
        "main",
        &json!({"at": "2026-11-05T11:00:01.000Z", "kind": "turn.started", "body": {"trigger": 7}}),
    );
    let items = kept(&hub, "g");
    assert_eq!(kinds(&items), ["heard", "started"]);
    assert_eq!(
        items[1],
        Item::Started {
            who: Some("小林".into()),
            glance: false
        },
        "开轮照触发的那一条找到名字"
    );
    assert_eq!(
        hub.venues(),
        [("g".to_string(), "Arch 群".to_string())],
        "场所的名字照群名"
    );
}

#[test]
fn deltas_only_count_while_someone_watches() {
    let hub = Arc::new(Hub::default());
    let start = json!({"kind": "model.delta", "body": {"seen": 3, "index": 0, "start": "text"}});
    let more = json!({"kind": "model.delta", "body": {"seen": 3, "index": 0, "text": "你好"}});
    hub.heard("g", "main", &start);
    assert!(hub.recent(None).is_empty(), "没人看，增量不整理");
    let (recent, mut pushed, watching) = hub.watch(Some("g"));
    assert!(recent.is_empty());
    hub.heard("g", "branch", &start);
    hub.heard("g", "branch", &more);
    hub.heard("h", "other", &more);
    let first = pushed.try_recv().expect("推了开头");
    let second = pushed.try_recv().expect("推了接上的");
    assert_eq!(
        kinds(
            &first
                .lines
                .iter()
                .map(|line| line.item.clone())
                .collect::<Vec<_>>()
        ),
        ["writing"]
    );
    assert_eq!(
        second.lines[0].item,
        Item::Appended {
            id: "branch:3:0".into(),
            text: "你好".into()
        },
        "支线的块带支线的编号"
    );
    assert!(pushed.try_recv().is_err(), "别的场所没人看，增量不推");
    assert!(hub.recent(None).is_empty(), "写着的不留");
    drop(watching);
    assert!(!hub.watched("g"), "放下就不算");
    hub.heard("g", "main", &more);
    assert!(pushed.try_recv().is_err());
}

#[test]
fn watching_everything_sees_every_place() {
    let hub = Arc::new(Hub::default());
    let (_, mut pushed, _watching) = hub.watch(None);
    hub.heard(
        "g",
        "main",
        &heard(1, "2026-11-05T11:00:00.000Z", "甲", "一"),
    );
    hub.heard("p", "dm", &heard(1, "2026-11-05T11:00:01.000Z", "乙", "二"));
    assert_eq!(pushed.try_recv().expect("g").venue, "g");
    assert_eq!(pushed.try_recv().expect("p").venue, "p");
}

#[test]
fn tool_names_are_learnt_from_results() {
    let hub = Hub::default();
    hub.heard(
        "g",
        "main",
        &json!({"kind": "message.assistant", "body": {"seen": 1, "blocks": [
        {"type": "tool_call", "call_id": "c1", "name": "qq_quote", "args": "{}"}]}}),
    );
    assert_eq!(hub.tool("qq_quote"), None);
    hub.heard(
        "g",
        "main",
        &json!({"kind": "tool.result", "body": {"call_id": "c1", "status": "ok",
        "display_name": {"zh": "引用", "en": "Quote"}, "blocks": []}}),
    );
    assert_eq!(hub.tool("qq_quote").as_deref(), Some("引用"));
    let items = kept(&hub, "g");
    assert!(
        matches!(&items[0], Item::Wrote { block: Block::Call(name), .. } if name == "qq_quote")
    );
}

#[test]
fn only_the_latest_lines_are_kept_and_merged_by_time() {
    let hub = Hub::default();
    for n in 0..(KEEP as u64 + 10) {
        let at = format!("2026-11-05T11:{:02}:{:02}.000Z", n / 60 % 60, n % 60);
        hub.heard("g", "main", &heard(n, &at, "甲", &n.to_string()));
    }
    hub.heard(
        "p",
        "dm",
        &heard(1, "2026-11-05T10:59:59.000Z", "乙", "最早"),
    );
    let lines = kept(&hub, "g");
    assert_eq!(lines.len(), KEEP);
    assert!(
        matches!(&lines[0], Item::Heard { text, .. } if text == "10"),
        "最旧的丢掉"
    );
    let all = hub.recent(None);
    assert_eq!(
        all.iter().map(|batch| batch.lines.len()).sum::<usize>(),
        KEEP,
        "合起来也只留这么多"
    );
    assert!(
        all.iter().all(|batch| batch.venue == "g"),
        "最早的那一条挤掉了"
    );
}

#[test]
fn places_are_kept_without_the_bot_number() {
    assert_eq!(
        super::short("qq:3927564101:group:130515298"),
        "qq:group:130515298"
    );
    assert_eq!(
        super::short("qq:3927564101:private:2433516202"),
        "qq:private:2433516202"
    );
    assert_eq!(
        super::short("tg:group:-100"),
        "tg:group:-100",
        "别的写法照原样"
    );
    let hub = Arc::new(Hub::default());
    let (_, mut pushed, _watching) = hub.watch(Some("qq:group:777"));
    hub.heard(
        "qq:1:group:777",
        "main",
        &heard(1, "2026-11-05T11:00:00.000Z", "甲", "一"),
    );
    assert_eq!(pushed.try_recv().expect("推了").venue, "qq:group:777");
}
