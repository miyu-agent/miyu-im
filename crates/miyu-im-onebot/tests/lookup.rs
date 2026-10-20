//! 头像和群成员（施工 O-54，`onebot.md` 第一条「头像和群成员」）：真核心拉起真桥、假 NapCat、她照台词调工具。`avatar` 照
//! `bridge.json` 的地址（测试的资源目录指到本机的假 HTTP 服务器）下下来、存进工作区的 `qq-files/`、答相对路径；`members` 照
//! `get_group_member_list` 列成一行、照名字筛。

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::json;

use miyu_http::testkit::{Piece, Reply, Server};

use crate::support::group::*;
use crate::support::media::*;
use crate::support::platform::*;
use crate::support::speaking::Lines;
use crate::support::*;

/// 假 HTTP 服务器回一次 `bytes`。
fn serving(bytes: &[u8]) -> Reply {
    Reply {
        status: 200,
        headers: vec![("Content-Type".to_string(), "image/png".to_string())],
        body: vec![Piece::Bytes(bytes.to_vec())],
    }
}

/// 存东西的 `qq-files/`（真实位置，`support::media::qq_files_in`）。
fn qq_files(home: &Home) -> PathBuf {
    crate::support::media::qq_files_in(home, "im")
}

#[tokio::test]
async fn an_avatar_is_saved_to_the_workspace() {
    let picture = png(3, 3);
    let server = Server::start(vec![serving(&picture), serving(&picture)]).await;
    let lines = Lines::new(
        [uses("avatar", r#"{"user":40001}"#), uses("avatar", "{}")]
            .into_iter()
            .flatten(),
    );
    let tuned = json!({
        "avatar_user_url": format!("{}/qq/{{id}}", server.base_url),
        "avatar_group_url": format!("{}/group/{{id}}", server.base_url),
    });
    let (home, napcat, _) = started_tuned(Arc::new(lines.clone()), &rules(), &tuned, |napcat| {
        napcat.ranked(MEMBERS, RANKS)
    })
    .await;
    // 终端管理员 @ 她，她取 40001 的头像。
    let words = json!([at(BOT), plain(" 看看他的头像")]);
    says(&home, &napcat, (ADMIN, "member"), 101, words).await;
    let got = ended(&home, 1).await;
    assert_eq!(
        got,
        done("saved", &[("path", "qq-files/avatar-qq-40001.png")])
    );
    assert_eq!(
        std::fs::read(qq_files(&home).join("avatar-qq-40001.png")).expect("存了"),
        picture
    );
    // 没写人：取叫她的人（终端管理员）。
    let words = json!([at(BOT), plain(" 我的头像")]);
    says(&home, &napcat, (ADMIN, "member"), 102, words).await;
    assert_eq!(
        ended(&home, 2).await,
        done(
            "saved",
            &[("path", &format!("qq-files/avatar-qq-{ADMIN}.png"))]
        )
    );
    let asked: Vec<String> = server
        .received()
        .iter()
        .map(|request| request.path.clone())
        .collect();
    assert_eq!(asked.len(), 2, "{asked:?}");
    assert!(asked[0].ends_with("/qq/40001"), "{asked:?}");
    assert!(asked[1].ends_with(&format!("/qq/{ADMIN}")), "{asked:?}");
    stopped(home).await;
}

#[tokio::test]
async fn members_are_listed_and_filtered_by_name() {
    let lines = Lines::new(
        [
            uses("members", "{}"),
            uses("members", r#"{"query":"群主"}"#),
            uses("members", r#"{"query":"没有"}"#),
        ]
        .into_iter()
        .flatten(),
    );
    let (home, napcat) = up(&lines).await;
    for (message, turn) in [(101, 1), (102, 2), (103, 3)] {
        let words = json!([at(BOT), plain(" 群里都有谁")]);
        says(&home, &napcat, (ADMIN, "member"), message, words).await;
        ended(&home, turn).await;
    }
    let results: Vec<(String, String)> = {
        let events = until_events(&home.root, &venue(), |events| {
            of_kind(events, "tool.result").len() == 3
        })
        .await;
        of_kind(&events, "tool.result")
            .iter()
            .map(|one| {
                (
                    one["body"]["blocks"][0]["text"]
                        .as_str()
                        .unwrap_or_default()
                        .trim_end()
                        .to_string(),
                    one["body"]["status"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                )
            })
            .collect()
    };
    let everyone: Vec<String> = MEMBERS
        .iter()
        .map(|(id, card, _)| {
            let role = RANKS
                .iter()
                .find(|(user, _)| user == id)
                .map_or("member", |(_, role)| role);
            format!("{id} {card} ({role})")
        })
        .collect();
    let total = MEMBERS.len().to_string();
    assert_eq!(
        results[0],
        done(
            "members",
            &[
                ("shown", &total),
                ("total", &total),
                ("list", &everyone.join("; "))
            ]
        )
    );
    assert_eq!(
        results[1],
        done(
            "members",
            &[
                ("shown", "1"),
                ("total", "1"),
                ("list", "40004 群主 (owner)")
            ]
        )
    );
    assert_eq!(results[2], done("no-members", &[]));
    stopped(home).await;
}
