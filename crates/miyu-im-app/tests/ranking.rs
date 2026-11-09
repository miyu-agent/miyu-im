//! 发言排行（施工 I-20）：真核心拉起真桥、假 NapCat。群里几个人说了几条，她调 `qq_activity_ranking`，im 照自己按人数的条数排，
//! 名字照群成员的缓存。

use serde_json::json;

use crate::support::group::*;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};

#[tokio::test]
async fn the_ranking_counts_who_talked_most() {
    let lines = Lines::new([
        Line::uses("", "qq_activity_ranking", r#"{"days":1}"#),
        Line::says("好。"),
    ]);
    let (home, napcat) = up(&lines).await;
    for message in 2001..2003 {
        says(
            &home,
            &napcat,
            (MEMBER, "member"),
            message,
            json!([plain("在吗")]),
        )
        .await;
    }
    says(
        &home,
        &napcat,
        (MANAGER, "member"),
        2003,
        json!([plain("嗯")]),
    )
    .await;
    says(
        &home,
        &napcat,
        (MEMBER, "member"),
        2004,
        called(json!([plain("谁最能说")])),
    )
    .await;
    let events = until_events(&home.root, &venue(), |events| {
        !of_kind(events, "turn.ended").is_empty()
    })
    .await;
    let result = of_kind(&events, "tool.result")[0]["body"]["blocks"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert_eq!(
        result.trim_end(),
        "Most messages in the last 1 day(s), top 2: 1. 小明 (40001): 3; 2. 群管 (40003): 1",
        "{events:#?}"
    );
    stopped(home).await;
}
