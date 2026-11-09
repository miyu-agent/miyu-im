//! 陌生人的私聊只记一行运行日志（`onebot.md` 第一条「怎么走」第 7 条、「施工时定的」第 49 条）：`venue.binding` 认不出是终端
//! 管理员的是陌生人（施工 I-65），同一个人连发几条，运行日志只记一行 `not admin or whitelisted, not taken`。测试那一头照核心拉起
//! 的桥的样子答 `venue.binding`。
//!
//! 全局装一个写进内存的订阅者，一个进程只能装一次，所以这个文件单独一个测试程序，只有一个测试。

mod support;

use miyu_log::{LevelFilter, Memory};
use miyu_session::testkit::{Play, Script};
use serde_json::json;

use support::*;

#[tokio::test]
async fn a_stranger_is_logged_once() {
    let memory = Memory::new();
    tracing::subscriber::set_global_default(miyu_log::subscriber(
        memory.clone(),
        LevelFilter::INFO,
        None,
    ))
    .expect("这个测试程序只装这一次");
    let home = Home::new(&Script::new([Play::Says("在")]));
    let bindings = Bindings {
        bound: &["qq:10001"],
    };
    let (serve, _relay) = serve_relayed(home.root.clone(), settings(), Some(bindings));
    let bridge = start(serve).await;
    let mut napcat = admin_napcat(bridge.port).await;
    for message in 1..=4 {
        napcat.private(STRANGER, message, json!("在吗")).await;
    }
    // 一条条照先后办：终端管理员这一句的回话先到，陌生人的四句就是办完了。
    napcat.admin_says(5, "在吗").await;
    assert_eq!(napcat.reply().await, "在");
    let refused: Vec<String> = memory
        .lines()
        .into_iter()
        .filter(|line| line.contains("not admin or whitelisted, not taken"))
        .collect();
    assert_eq!(refused.len(), 1, "同一个人只记一行：{refused:?}");
    assert!(
        refused[0].contains("venue=qq:30003:private:20002"),
        "{refused:?}"
    );
    bridge.stop().await.expect("停得下");
}
