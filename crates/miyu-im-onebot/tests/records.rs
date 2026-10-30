//! 聊天记录（设计 04，施工 C-1 上）：真核心照开关拉起真桥、假 NapCat。群里别人说的（没叫她也记）、她回的、撤回（标上、不删）、她
//! 被禁言、群里别人被禁言和踢（施工 C-1 上补，一件件事），都记进 im 自己的库（`state/packages/miyu-im/records.db`），带的东西只记占位和编号。

use std::time::Duration;

use rusqlite::Connection;
use serde_json::json;

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::*;

/// 群号。
const GROUP: i64 = 3951;

/// 群里的别人。
const LIN: i64 = 20002;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 系统的场所规则：别人没叫她的只记下。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\n";

/// 照 `sql` 数一个数，等到合 `wanted`（写的线程排着写）。
async fn until(home: &Home, sql: &str, wanted: i64) -> i64 {
    let path = home
        .root
        .state()
        .join("packages")
        .join("miyu-im")
        .join("records.db");
    within("聊天记录记上了", async {
        loop {
            let counted = Connection::open(&path)
                .and_then(|db| db.query_row(sql, [], |row| row.get::<_, i64>(0)))
                .unwrap_or(-1);
            if counted == wanted {
                return counted;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
}

#[tokio::test]
async fn messages_her_words_recalls_and_mutes_are_recorded() {
    let script = Script::new([Play::Says("在呢")]);
    let (home, mut napcat, _) = started(&script, RULES, "", MEMBERS).await;
    let venue = format!("qq:30003:group:{GROUP}");
    // 小林说一句、带一张图（没叫她：只记下，聊天记录照记）。
    napcat.send(group_frame(
        GROUP,
        LIN,
        1,
        json!([plain("今天晚上一起打团本吗"), {"type": "image", "data": {"file": "abc.jpg", "file_size": "1024"}}]),
        ("小林", "lin"),
    ));
    // 终端管理员 @ 她，她回一句。
    napcat.send(group_frame(
        GROUP,
        ADMIN,
        2,
        json!([at(BOT), plain(" 在吗")]),
        ("终端管理员", "o"),
    ));
    assert_eq!(napcat.group_reply(GROUP).await, "在呢");
    until(&home, "SELECT count(*) FROM messages WHERE mine = 1", 1).await;
    until(&home, "SELECT count(*) FROM messages WHERE mine = 0", 2).await;
    until(
        &home,
        "SELECT count(*) FROM messages WHERE text LIKE '%打团本%' AND sender = 'qq:20002' AND name = '小林' AND media = '[{\"id\":\"abc.jpg\",\"kind\":\"image\",\"size\":1024}]'",
        1,
    )
    .await;
    // 小林撤回了那一条：标上、不删，另记一件事。
    napcat.send(group_recall(GROUP, LIN, LIN, 1));
    until(
        &home,
        "SELECT count(*) FROM messages WHERE msg = '1' AND recalled_by = 'qq:20002' AND text LIKE '%打团本%'",
        1,
    )
    .await;
    // 她被禁言了：一件事。
    napcat.send(group_ban(GROUP, BOT, "ban", Some(600)));
    until(
        &home,
        &format!("SELECT count(*) FROM happenings WHERE venue = '{venue}' AND kind = 'muted' AND seconds = 600 AND target = 'qq:{BOT}'"),
        1,
    )
    .await;
    // 群里别的管理员禁言了小林、又把他踢了（施工 C-1 上补）：两件事，谁做的、对谁都记。
    napcat.send(group_ban(GROUP, LIN, "ban", Some(60)));
    napcat.send(json!({"time": TIME, "self_id": BOT, "post_type": "notice", "notice_type": "group_decrease", "sub_type": "kick", "group_id": GROUP, "user_id": LIN, "operator_id": 40004}));
    until(
        &home,
        &format!("SELECT count(*) FROM happenings WHERE venue = '{venue}' AND target = 'qq:{LIN}' AND ((kind = 'muted' AND seconds = 60) OR (kind = 'kicked' AND actor = 'qq:40004'))"),
        2,
    )
    .await;
    stopped(home).await;
}
