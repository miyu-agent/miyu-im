//! 查用量（施工 I-206，项目主人 11-10：群聊里 AI 没法查 token 用量）：真核心拉起真桥、假 NapCat。群友叫她一轮，再叫她查：
//! `qq_token_usage` 不写 `user` 的答这个群这段时间的合计（照核心的账本）和用得最多的人（照 im 照人记的账本）；写了 `user` 的答
//! 那个人的；认不出是谁的答写错了。测试的模型没有价钱：金额那一格写没记录。

use serde_json::json;

use crate::support::group::*;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};

/// 这个群的会话里工具结果的字，照先后。
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

#[tokio::test]
async fn the_group_and_one_member_can_be_asked_about() {
    let lines = Lines::new([
        Line::says("在。"),
        Line::uses("", "qq_token_usage", "{}"),
        Line::says("好。"),
        Line::uses("", "qq_token_usage", r#"{"user":"小明","hours":48}"#),
        Line::says("好。"),
        Line::uses("", "qq_token_usage", r#"{"user":"没这个人"}"#),
        Line::says("好。"),
    ]);
    let (home, napcat) = up(&lines).await;
    let ask = |message: i64, text: &'static str| {
        says(
            &home,
            &napcat,
            (MEMBER, "member"),
            message,
            called(json!([plain(text)])),
        )
    };
    ask(2101, "在吗").await;
    results(&home, 1).await;
    // 一轮一轮来：连着发的话后一条会并进、打断前一轮。
    ask(2102, "这个群用了多少").await;
    results(&home, 2).await;
    ask(2103, "我用了多少").await;
    results(&home, 3).await;
    ask(2104, "他用了多少").await;
    let got = results(&home, 4).await;
    assert_eq!(got.len(), 3, "{got:#?}");
    let whole = &got[0];
    assert!(
        whole.starts_with("Last 24h this chat used ")
            && whole.contains(" requests, no cost recorded"),
        "合计照核心的账本，测试的模型没有价钱：{whole}"
    );
    assert!(
        whole.contains("Top callers: 小明 "),
        "用得最多的人照名字写：{whole}"
    );
    assert!(!whole.contains("used 0 tokens"), "{whole}");
    let mine = &got[1];
    assert!(
        mine.starts_with("Last 48h 小明 used ")
            && mine.contains(" tokens here, no cost recorded for "),
        "{mine}"
    );
    assert!(
        got[2].contains("user must be a QQ number or a name"),
        "{}",
        got[2]
    );
    stopped(home).await;
}
