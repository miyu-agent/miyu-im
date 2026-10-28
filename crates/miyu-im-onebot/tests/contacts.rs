//! 通讯录、群文件、贴表情（施工 I-12）：真核心拉起真桥、假 NapCat、她照台词调工具。通讯录照 `get_friend_list`、`get_group_list`
//! 列、照名字筛；群文件照 `get_group_root_files` 列一层，给了编号的照 `get_group_file_url` 的地址下下来存进工作区的 `qq-files/`，
//! 名字照地址里的 `fname`；贴表情先核对那一条在这个群，再 `set_msg_emoji_like`，表情换成码点的十进制。

use std::sync::Arc;

use serde_json::json;

use miyu_http::testkit::{Piece, Reply, Server};

use crate::support::group::*;
use crate::support::platform::*;
use crate::support::speaking::Lines;

#[tokio::test]
async fn contacts_group_files_and_reactions_go_through_qq() {
    let server = Server::start(vec![Reply {
        status: 200,
        headers: vec![("Content-Type".to_string(), "application/pdf".to_string())],
        body: vec![Piece::Bytes(b"%PDF-1.4 schedule".to_vec())],
    }])
    .await;
    let lines = Lines::new(
        [
            uses("qq_contacts", r#"{"query":"ming"}"#),
            uses("qq_contacts", r#"{"kind":"groups"}"#),
            uses("qq_group_files", "{}"),
            uses("qq_group_files", r#"{"file":"/f1"}"#),
            uses("qq_react", r#"{"msg":"777","emoji":"👍"}"#),
            uses("qq_react", r#"{"msg":"778","emoji":"👍"}"#),
        ]
        .into_iter()
        .flatten(),
    );
    let (home, mut napcat, _) = started_ranked(
        Arc::new(lines.clone()),
        &rules(),
        &onebot(),
        (MEMBERS, RANKS),
    )
    .await;
    napcat.stock_file(
        "root",
        Ok(json!({"folders": [{"folder_id": "/a1", "folder_name": "资料", "total_file_count": 3}],
                  "files": [{"file_id": "/f1", "file_name": "排班.pdf", "size": 2048, "uploader_name": "小明", "busid": 102}]})),
    );
    napcat.stock_file(
        "/f1",
        Ok(json!({"url": format!("{}/d?fname=排班.pdf", server.base_url)})),
    );
    napcat.stock_message(777, json!({"message_id": 777, "group_id": GROUP, "message_type": "group", "sender": {"user_id": MEMBER}, "message": []}));
    napcat.stock_message(778, json!({"message_id": 778, "group_id": 999, "message_type": "group", "sender": {"user_id": MEMBER}, "message": []}));
    let words = json!([at(MEMBER), plain(" 帮个忙")]);
    assert_eq!(
        asks(&home, &napcat, MANAGER, 1201, words.clone(), 1).await,
        done(
            "contacts",
            &[
                ("shown", "1"),
                ("total", "1"),
                ("kind", "friends"),
                ("list", "40001 ming")
            ]
        )
    );
    let (groups, _) = asks(&home, &napcat, MANAGER, 1202, words.clone(), 2).await;
    assert!(groups.contains("groups"), "{groups}");
    assert_eq!(
        asks(&home, &napcat, MANAGER, 1203, words.clone(), 3).await,
        done(
            "group-files",
            &[
                ("shown", "2"),
                ("total", "2"),
                (
                    "list",
                    "[folder] 资料 (id=/a1, 3 files); 排班.pdf (id=/f1, 2.0 KB, by 小明)"
                )
            ]
        )
    );
    assert_eq!(
        asks(&home, &napcat, MANAGER, 1204, words.clone(), 4).await,
        done("saved", &[("path", "qq-files/排班.pdf")])
    );
    let saved = crate::support::media::qq_files_in(&home, "admin").join("排班.pdf");
    assert_eq!(std::fs::read(&saved).expect("存了"), b"%PDF-1.4 schedule");
    assert_eq!(
        asks(&home, &napcat, MANAGER, 1205, words.clone(), 5).await,
        done("reacted", &[])
    );
    let reacted = napcat.reacted().await;
    assert_eq!(
        reacted["message_id"].to_string().trim_matches('"'),
        "777",
        "{reacted}"
    );
    assert_eq!(
        reacted["emoji_id"].to_string().trim_matches('"'),
        "128077",
        "{reacted}"
    );
    assert_eq!(
        asks(&home, &napcat, MANAGER, 1206, words.clone(), 6).await,
        refused("not-found", &[]),
        "别的群的那一条"
    );
    stopped(home).await;
}
