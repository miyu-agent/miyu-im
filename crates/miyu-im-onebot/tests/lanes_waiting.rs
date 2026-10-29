//! 排着的人出队（施工 I-59，10-27 项目主人在群里撞到她一句话 @ 了两个人）：主线在回 A、唯一的支线在回 B，C 排着；A 追问一句，
//! 核心把它排在主线后面，这一轮一结束就照它开下一轮。桥出队前先收完留着的推送，看见主线又在回 A，C 接着排着，不并进 A 那一轮；
//! A 那一轮完了 C 才单开一轮。真核心拉起真桥、假 NapCat、她照台词说（`support/speaking.rs`）。

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::oneshot;

use crate::support::group::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 群号。
const GROUP: i64 = 3603;

/// 群里的几个人。
const A: i64 = 20031;
const B: i64 = 20032;
const C: i64 = 20033;

/// 假 NapCat 认得的群成员。
const MEMBERS: &[Member] = &[
    (BOT, "米尤", "miyu"),
    (A, "阿甲", "a"),
    (B, "阿乙", "b"),
    (C, "阿丙", "c"),
];

/// 系统的场所规则：叫她才回（不过判官），最多一条支线，顶替窗口很短（追问并进来而不是打断重来，施工 I-21）。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\ndiscipline = \"when-called\"\nrate = \"100/1h\"\nparallel = 1\ndispatch = { supersede_window = \"1s\" }\n";

/// `user` @ 她说第 `message` 条。
fn calls(napcat: &Answering, user: i64, message: i64, text: &str) {
    let (_, card, nickname) = MEMBERS
        .iter()
        .find(|(id, _, _)| *id == user)
        .expect("认得的成员");
    napcat.send(group_frame(
        GROUP,
        user,
        message,
        json!([at(BOT), plain(&format!(" {text}"))]),
        (card, nickname),
    ));
}

/// 等到她收到第 `n` 次主请求。
async fn until_requests(lines: &Lines, n: usize) {
    within("她收到请求", async {
        while lines.requests().len() < n {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
}

/// 主线每一轮的触发（`turn.started` 的、`turn.joined` 并进来的）的发的人，照回合编号先后。
fn turn_people(events: &[Value]) -> Vec<Vec<String>> {
    let sender = |seq: &Value| {
        events
            .iter()
            .find(|event| event["seq"] == *seq)
            .and_then(|event| event["by"]["id"].as_str())
            .unwrap_or_default()
            .to_string()
    };
    let mut turns: Vec<(u64, Vec<String>)> = Vec::new();
    for event in events {
        let Some(turn) = event["turn"].as_u64() else {
            continue;
        };
        let triggers = match event["kind"].as_str() {
            Some("turn.started" | "turn.joined") => event["body"]["triggers"].as_array(),
            _ => None,
        };
        let Some(triggers) = triggers else { continue };
        let people: Vec<String> = triggers.iter().map(sender).collect();
        match turns.iter_mut().find(|(one, _)| *one == turn) {
            Some((_, all)) => all.extend(people),
            None => turns.push((turn, people)),
        }
    }
    turns.into_iter().map(|(_, people)| people).collect()
}

#[tokio::test]
async fn a_queued_caller_is_not_folded_into_a_turn_the_core_just_started_for_someone_else() {
    let (release_a, hold_a) = oneshot::channel();
    let (release_b, hold_b) = oneshot::channel();
    let lines = Lines::new([
        Line::says("回A").released_by(hold_a),
        Line::says("回B").released_by(hold_b),
        Line::says("再回A"),
        Line::says("回C"),
    ]);
    let (home, mut napcat, _) = started_by(Arc::new(lines.clone()), RULES, MEMBERS).await;
    let venue = format!("qq:30003:group:{GROUP}");
    // 主线回 A（压着），支线回 B（压着），C 排着。
    calls(&napcat, A, 101, "在吗");
    until_requests(&lines, 1).await;
    calls(&napcat, B, 201, "有空吗");
    until_requests(&lines, 2).await;
    calls(&napcat, C, 301, "帮个忙");
    until_logged(&home.root, |log| log.contains("reply queued")).await;
    // 过了顶替窗口 A 追问：核心排在主线后面，这一轮一结束就照它开下一轮。
    tokio::time::sleep(Duration::from_millis(1100)).await;
    calls(&napcat, A, 102, "？");
    until_logged(&home.root, |log| log.matches("respond asked").count() >= 3).await;
    release_a.send(()).expect("还在等");
    let mut heard = Vec::new();
    for _ in 0..3 {
        heard.push(napcat.group_message(GROUP).await);
    }
    release_b.send(()).expect("还在等");
    let events = until_events(&home.root, &venue, |events| {
        of_kind(events, "turn.ended").len() >= 3
    })
    .await;
    let people = turn_people(&events);
    let (a, c) = (format!("qq:{A}"), format!("qq:{C}"));
    assert!(
        people
            .iter()
            .all(|turn| !(turn.contains(&a) && turn.contains(&c))),
        "C 不并进 A 那一轮：{people:?}\n{}",
        run_log(&home.root)
    );
    assert!(
        people.iter().any(|turn| *turn == [c.clone()]),
        "C 单开一轮：{people:?}"
    );
    assert_eq!(heard.len(), 3, "{heard:?}");
    stopped(home).await;
}
