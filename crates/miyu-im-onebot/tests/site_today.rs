//! 首页「今天」的几个数和最近的运行日志（施工 I-6 下）：真核心照开关拉起真桥，群里一条只记下、一条叫了她她回了；页面经通道问
//! `today`、`logs`。

use serde_json::json;

use miyu_session::testkit::{Play, Script};

use crate::support::browser::{call, logged_in};
use crate::support::group::*;
use crate::support::*;

/// 群号。
const GROUP: i64 = 3931;

/// 群里的人。
const LIN: i64 = 20004;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 系统的场所规则：只有一条配群的空规则。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\n";

#[tokio::test]
async fn the_home_page_counts_today_and_shows_the_log() {
    let script = Script::new([Play::Says("在的。")]);
    let (home, mut napcat, _) = started(&script, RULES, "", MEMBERS).await;
    napcat.send(group_frame(
        GROUP,
        LIN,
        1,
        json!([plain("大家好")]),
        ("小林", "lin"),
    ));
    napcat.send(group_frame(
        GROUP,
        LIN,
        2,
        json!([at(BOT), plain(" 在吗")]),
        ("小林", "lin"),
    ));
    assert_eq!(napcat.group_reply(GROUP).await, "在的");
    let board = miyu_im::status_file::read(&home.root).expect("写了状态文件");
    let port = u16::try_from(board["web"].as_u64().expect("页面的端口")).expect("是端口");
    let mut open = logged_in(&home.root, port).await;
    let today = within("数到了", async {
        let mut asked = 1;
        loop {
            let today = call(&mut open, asked, "today", json!({})).await["result"].clone();
            if today["replied"] == 1 {
                return today;
            }
            asked += 1;
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    assert_eq!(
        today,
        json!({"received": 2, "replied": 1, "recorded": 1, "undelivered": 0})
    );
    let logs = call(&mut open, 100, "logs", json!({"limit": 500})).await["result"]["logs"].clone();
    let lines = logs.as_array().expect("是列表");
    assert!(
        !lines.is_empty() && lines.len() <= 200,
        "最多 200 行：{}",
        lines.len()
    );
    assert!(
        lines.iter().any(|line| line["level"] == "INFO"
            && line["text"]
                .as_str()
                .is_some_and(|text| text.starts_with("reply sent"))),
        "{logs}"
    );
    let warned =
        call(&mut open, 101, "logs", json!({"level": "warn"})).await["result"]["logs"].clone();
    assert!(
        warned
            .as_array()
            .expect("是列表")
            .iter()
            .all(|line| line["level"] == "WARN" || line["level"] == "ERROR"),
        "{warned}"
    );
    stopped(home).await;
}
