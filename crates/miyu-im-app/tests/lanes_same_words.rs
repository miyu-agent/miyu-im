//! 两条线说了一样的话只发一句（施工 I-217）：线上 2026-11-11 两个人隔 2 秒 @ 她，主线和支线各回各的，两边说的都是「这个也不说」，
//! 群里出了两句一模一样的。真核心拉起真桥、假 NapCat、她照台词说（`support/speaking.rs`）。

use std::sync::Arc;

use serde_json::json;
use tokio::sync::oneshot;

use crate::support::group::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 群号。
const GROUP: i64 = 3617;

/// 群里的两个人。
const A: i64 = 20011;
const B: i64 = 20012;

/// 假 NapCat 认得的群成员。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (A, "阿甲", "a"), (B, "阿乙", "b")];

/// 系统的场所规则：叫她才回（不过判官），一条支线。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\ndiscipline = \"when-called\"\nrate = \"100/1h\"\nparallel = 1\n";

/// 场所编号。
fn venue() -> String {
    format!("qq:30003:group:{GROUP}")
}

/// `user` @ 她说第 `message` 条。
fn calls(napcat: &Answering, user: i64, message: i64, text: &str) {
    let (_, card, nickname) = MEMBERS
        .iter()
        .find(|(id, _, _)| *id == user)
        .expect("认得的成员");
    let words = json!([at(BOT), plain(&format!(" {text}"))]);
    napcat.send(group_frame(GROUP, user, message, words, (card, nickname)));
}

/// 等到她收到第 `n` 次主请求。
async fn until_requests(lines: &Lines, n: usize) {
    within("她收到请求", async {
        while lines.requests().len() < n {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
}

/// 她发进群里的下一句的字。
async fn next_said(napcat: &mut Answering) -> String {
    let message = napcat.group_message(GROUP).await;
    message.last().expect("有段")["data"]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[tokio::test]
async fn the_same_words_from_two_lines_reach_the_group_once() {
    let (release_a, hold_a) = oneshot::channel();
    let lines = Lines::new([
        Line::says("在"),
        Line::says("这个也不说").released_by(hold_a),
        Line::says("这个也不说"),
        Line::says("这个可以说"),
    ]);
    let (home, mut napcat, _) = started_by(Arc::new(lines.clone()), RULES, MEMBERS).await;
    calls(&napcat, A, 101, "在吗");
    assert_eq!(next_said(&mut napcat).await, "在");
    until_events(&home.root, &venue(), |events| {
        !of_kind(events, "turn.ended").is_empty()
    })
    .await;
    // 主线回 A，压着；B @ 她：分叉一条支线回他，支线先说了那一句。
    calls(&napcat, A, 102, "屑");
    until_requests(&lines, 2).await;
    calls(&napcat, B, 201, "之前后端是什么");
    until_requests(&lines, 3).await;
    assert_eq!(next_said(&mut napcat).await, "这个也不说");
    // 放主线：它说的是一模一样的一句，别的线刚说过，不发，记一行。
    release_a.send(()).expect("还在等");
    until_logged(&home.root, |log| {
        log.lines()
            .any(|line| line.contains("reply dropped") && line.contains("why=repeated"))
    })
    .await;
    // 群里下一句是她后来回的，中间没有第二句「这个也不说」。
    until_events(&home.root, &venue(), |events| {
        of_kind(events, "turn.ended").len() >= 2
    })
    .await;
    calls(&napcat, A, 103, "那今天星期几");
    assert_eq!(next_said(&mut napcat).await, "这个可以说");
    stopped(home).await;
}
