//! 查聊天记录（施工 C-1 下，设计 04 第一节第 4 条）：真核心拉起真桥、假 NapCat，她照台词调 `qq_chat_history`。群里说过的照关键词搜得到，
//! 带 `msg=` 编号；跨场所查只有终端管理员叫的那一轮能查，普通成员叫的答只给管理员。

use serde_json::json;

use crate::support::group::*;
use crate::support::platform::*;
use crate::support::speaking::Lines;
use crate::support::*;

#[tokio::test]
async fn she_searches_this_chat_and_only_an_admin_turn_searches_every_chat() {
    let lines = Lines::new(
        [
            uses("qq_chat_history", r#"{"query":"团本"}"#),
            uses("qq_chat_history", r#"{"all_chats":true}"#),
            uses("qq_chat_history", r#"{"all_chats":true,"user":"小明"}"#),
        ]
        .into_iter()
        .flatten(),
    );
    let (home, napcat) = up(&lines).await;
    says(
        &home,
        &napcat,
        (MEMBER, "member"),
        101,
        json!([plain("今晚打团本吗")]),
    )
    .await;
    // 普通成员叫她查：只查这个群，搜到小明那一条，带编号。
    let (text, status) = asks(
        &home,
        &napcat,
        IMPOSTOR,
        102,
        json!([plain("刚才谁说了啥")]),
        1,
    )
    .await;
    assert_eq!(status, "ok", "{text}");
    let mut rows = text.lines();
    assert_eq!(
        rows.next(),
        Some(sentence("history", &[("count", "1")]).as_str())
    );
    let row = rows.next().expect("有一行");
    assert!(
        row.ends_with(" 小明 (40001) msg=101: 今晚打团本吗"),
        "{row}"
    );
    assert_eq!(rows.next(), None, "{text}");
    // 普通成员的那一轮要跨场所查：不查。
    assert_eq!(
        asks(
            &home,
            &napcat,
            IMPOSTOR,
            103,
            json!([plain("查查所有群")]),
            2
        )
        .await,
        refused("history-admin-only", &[])
    );
    // 终端管理员叫的那一轮：跨场所查，每行写哪个场所。
    let (text, status) = asks(
        &home,
        &napcat,
        ADMIN,
        104,
        json!([plain("小明在各处说过啥")]),
        3,
    )
    .await;
    assert_eq!(status, "ok", "{text}");
    assert!(
        text.lines()
            .any(|row| row.ends_with(" group 31, 小明 (40001) msg=101: 今晚打团本吗")),
        "{text}"
    );
    stopped(home).await;
}
