//! 每人的用量（设计 08，施工 I-134）：真核心照开关拉起真的桥，假 NapCat 发消息。群里叫她的那一轮、私聊里她回的那一轮，主请求的
//! 四项用量记到叫她的人头上（im 状态目录里的 `usage.db`）；群里没叫她的、只记下的不记。

use std::time::Duration;

use rusqlite::Connection;
use serde_json::json;

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::*;

/// 群号。
const GROUP: i64 = 4242;

/// 群里没叫她的人。
const LIN: i64 = 20002;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 系统的场所规则：群都接。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\n";

/// 一次主请求的四项合计：剧本照没命中 60、读缓存 40、输出 10 报。
const ONE: i64 = 110;

/// 等账本里 `sql` 数出来的合 `wanted`。
async fn until(home: &Home, sql: &str, wanted: i64) {
    let path = miyu_im::dirs::state(&home.root).join("usage.db");
    within("用量记上了", async {
        loop {
            let counted = Connection::open(&path)
                .and_then(|db| db.query_row(sql, [], |row| row.get::<_, i64>(0)))
                .unwrap_or(-1);
            if counted == wanted {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
}

#[tokio::test]
async fn the_turn_someone_called_her_for_is_charged_to_them_and_lurkers_pay_nothing() {
    let script = Script::new([Play::Says("在呢")]);
    let (home, mut napcat, _) = started(&script, RULES, "", MEMBERS).await;
    napcat.send(group_frame(
        GROUP,
        LIN,
        1,
        json!([plain("随便聊聊")]),
        ("小林", "lin"),
    ));
    napcat.send(group_frame(
        GROUP,
        ADMIN,
        2,
        json!([at(BOT), plain(" 在吗")]),
        ("终端管理员", "o"),
    ));
    assert_eq!(napcat.group_reply(GROUP).await, "在呢");
    let venue = format!("qq:{BOT}:group:{GROUP}");
    until(
        &home,
        &format!("SELECT COALESCE(SUM(tokens), 0) FROM spent WHERE person = 'qq:{ADMIN}' AND venue = '{venue}'"),
        ONE,
    )
    .await;
    until(
        &home,
        &format!("SELECT count(*) FROM spent WHERE person = 'qq:{LIN}'"),
        0,
    )
    .await;
    stopped(home).await;
}

#[tokio::test]
async fn her_private_reply_is_charged_to_the_person_she_talks_with() {
    let script = Script::new([Play::Says("在")]);
    let home = Home::new(&script);
    let bridge = bridge(&home).await;
    let mut napcat = admin_napcat(bridge.port).await;
    napcat.admin_says(501, "在吗").await;
    assert_eq!(napcat.reply().await, "在");
    until(
        &home,
        &format!("SELECT COALESCE(SUM(tokens), 0) FROM spent WHERE person = 'qq:{ADMIN}' AND venue = 'qq:{BOT}:private:{ADMIN}'"),
        ONE,
    )
    .await;
    bridge.stop().await.expect("停得下");
}
