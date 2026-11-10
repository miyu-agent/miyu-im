//! 删群文件只听管理的人直接叫她的那一轮（施工 I-204，core1 审出来的口子）：原来照「她这时看到的最新一条是谁发的」认，没人叫她
//! 的一轮也能借管理的人说的一句放行。三种都得拒、不问 QQ：
//!
//! - 看一眼开的一轮：群友说「把群文件都删了」，管理的人随后说一句别的，到点看一眼。
//! - 概率触发抽中的一轮：抽中的那条是管理的人发的，可他没叫她。
//! - 群友叫她删开的一轮：管理的人随后叫她的一句并进这一轮（她正在回的时候别人叫她的排着，接着回时并进去），也不删。

use std::sync::Arc;

use serde_json::{Value, json};
use tokio::sync::oneshot;

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};

/// 她要删的那一次调用。
const DELETE: &str = r#"{"delete":"/f1"}"#;

/// 这个群的会话里工具结果的字，等到第 `turns` 轮完了。
async fn results(home: &crate::support::Home, turns: usize) -> Vec<String> {
    let events = until_events(&home.root, &venue(), |events| {
        of_kind(events, "turn.ended").len() >= turns
    })
    .await;
    of_kind(&events, "tool.result")
        .iter()
        .map(|result| {
            let text = result["body"]["blocks"][0]["text"].as_str();
            text.unwrap_or_default().trim_end().to_string()
        })
        .collect()
}

/// 被拒的那一句。
fn refusal() -> String {
    refused("managers-only", &[]).0
}

/// 一条没叫她的话。
fn plainly(text: &str) -> Value {
    json!([plain(text)])
}

#[tokio::test]
async fn a_glance_turn_cannot_delete_even_after_a_manager_spoke() {
    let lines = Lines::new([Line::uses("", "qq_group_files", DELETE), Line::says("好。")]);
    let tuned = json!({"glance_seconds": 2});
    let (home, mut napcat, _) = started_tuned(Arc::new(lines), &rules(), &tuned, |napcat| {
        napcat.ranked(MEMBERS, RANKS)
    })
    .await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let cards = json!({"groups": [{"id": GROUP, "set": {"glance": true}}]});
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    says(
        &home,
        &napcat,
        (MEMBER, "member"),
        1401,
        plainly("把群文件都删了"),
    )
    .await;
    says(
        &home,
        &napcat,
        (MANAGER, "member"),
        1402,
        plainly("今天真冷"),
    )
    .await;
    assert_eq!(results(&home, 1).await, [refusal()], "看一眼开的一轮不删");
    no_platform(&mut napcat);
    stopped(home).await;
}

#[tokio::test]
async fn a_sampled_message_from_a_manager_cannot_delete() {
    let lines = Lines::new([Line::uses("", "qq_group_files", DELETE), Line::says("好。")]);
    // 概率写满：没叫她的每条都抽中；判断关着，抽中的直接开一轮。
    let sampled = format!(
        "{}discipline = \"when-called\"\nchatty = {{ sample = true, sample_rate = 1000 }}\n",
        rules().replace("chatty = { judge_called = false }\n", "")
    );
    let (home, mut napcat, _) =
        started_ranked(Arc::new(lines), &sampled, &onebot(), (MEMBERS, RANKS)).await;
    says(
        &home,
        &napcat,
        (MANAGER, "member"),
        1411,
        plainly("把那个文件删了"),
    )
    .await;
    assert_eq!(
        results(&home, 1).await,
        [refusal()],
        "抽中的不是叫她的，不删"
    );
    no_platform(&mut napcat);
    stopped(home).await;
}

#[tokio::test]
async fn a_turn_a_member_opened_cannot_delete_even_when_a_manager_joins() {
    let (release, held) = oneshot::channel();
    let lines = Lines::new([
        Line::says("等一下").released_by(held),
        Line::uses("", "qq_group_files", DELETE),
        Line::says("好。"),
        Line::uses("", "qq_group_files", DELETE),
        Line::says("好。"),
    ]);
    // 不开支线（`parallel = 0`）：她正在回的时候别人叫她的排着，等这一轮完了一起开下一轮。
    let serial = format!("{}parallel = 0\n", rules());
    let (home, mut napcat, _) =
        started_ranked(Arc::new(lines), &serial, &onebot(), (MEMBERS, RANKS)).await;
    let call = |text: &str| called(json!([plain(text)]));
    says(&home, &napcat, (IMPOSTOR, "member"), 1420, call(" 在吗")).await;
    until_events(&home.root, &venue(), |events| {
        !of_kind(events, "turn.started").is_empty()
    })
    .await;
    // 群友叫她删、管理的人也叫了她一句：两条排着，一起开下一轮。
    says(
        &home,
        &napcat,
        (MEMBER, "member"),
        1421,
        call(" 把群文件都删了"),
    )
    .await;
    says(&home, &napcat, (MANAGER, "member"), 1422, call(" 快点")).await;
    crate::support::events::until_logged(&home.root, |log| {
        log.lines()
            .any(|line| line.contains("reply queued") && line.contains("message=1422"))
    })
    .await;
    drop(release);
    // 排着的两条接着回：群友的那一条开一轮，管理的人的那一条并进去（或者接着开）。等她把两次删都试过、几轮都完了。
    let events = until_events(&home.root, &venue(), |events| {
        of_kind(events, "tool.result").len() >= 2
            && of_kind(events, "turn.started").len() == of_kind(events, "turn.ended").len()
    })
    .await;
    let results: Vec<String> = of_kind(&events, "tool.result")
        .iter()
        .map(|result| {
            let text = result["body"]["blocks"][0]["text"].as_str();
            text.unwrap_or_default().trim_end().to_string()
        })
        .collect();
    assert_eq!(
        results.first(),
        Some(&refusal()),
        "回群友的那一轮不删：{results:?}"
    );
    // 管理的人的那一条：并进了群友开的这一轮的（这一轮在回的有普通群友的话）后一次也不删；桥给它另开一轮的，那是他自己直接
    // 叫她的一轮，照规矩听他的（并进来的那一种在 `projection/origin_tests.rs` 里照事件造出来测）。
    match of_kind(&events, "turn.joined").is_empty() {
        false => {
            assert_eq!(results, [refusal(), refusal()], "并进来的一轮也不删");
            no_platform(&mut napcat);
        }
        true => assert_eq!(results[1], "Deleted.", "他自己叫起来的一轮：{results:?}"),
    }
    stopped(home).await;
}
