//! 核心压测以后新加的几种拒绝，桥照着办（施工 YC-10）：
//!
//! - 场所会话的日志中间坏了一行、只能看（`session_read_only`，核心 YC-6）：桥另开一个新的接着用（项目主人 2026-11-10 定），
//!   这一条消息照样说进去、她照样回；旧的留着。原来这个场所从此不出声。
//! - 会话这一刻读不了（`session_unavailable`，核心 YC-2）等一会儿再试：这里没测，测试里造不出来（把日志的权限去掉，核心照样
//!   答得了）。
//! - 同一个命令编号参数变了（`id_reused`，核心 YC-7）：是另一条消息，换一个编号说进去，不当成重发丢掉。

use std::path::PathBuf;
use std::time::Duration;

use miyu_session::testkit::{Play, Script};

use crate::support::*;

/// 等终端管理员的私聊的会话说完 `turns` 轮，停下全部会话（像核心重启过），交回它唯一的那一段日志。
async fn stopped_segment(home: &Home, turns: usize) -> PathBuf {
    within("这一轮说完", async {
        loop {
            let ended = home
                .sessions()
                .iter()
                .flat_map(|session| home.events(session))
                .filter(|event| event["kind"] == "turn.ended")
                .count();
            if ended >= turns {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    home.stop_sessions().await;
    let sessions = home.sessions();
    assert_eq!(sessions.len(), 1, "{sessions:?}");
    let segments: Vec<PathBuf> = std::fs::read_dir(home.root.session_dir(&sessions[0]))
        .expect("会话目录在")
        .map(|entry| entry.expect("读得了").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .collect();
    assert_eq!(segments.len(), 1, "{segments:?}");
    segments[0].clone()
}

#[tokio::test]
async fn a_venue_whose_session_log_broke_goes_on_in_a_fresh_session() {
    let script = Script::new([Play::Says("一"), Play::Says("二"), Play::Says("三")]);
    let home = Home::new(&script);
    let first = bridge(&home).await;
    let mut napcat = admin_napcat(first.port).await;
    napcat.admin_says(1, "头一句").await;
    assert_eq!(napcat.reply().await, "一");
    // 桥停着的时候坏的（像整机重启过）：桥连着的话，会话一停它就再订阅、又载入了，日志坏不到没载入的会话上。
    drop(napcat);
    first.stop().await.expect("停得下");
    let segment = stopped_segment(&home, 1).await;
    let text = std::fs::read_to_string(&segment).expect("读得了");
    let mut lines: Vec<&str> = text.lines().collect();
    let middle = lines.len() / 2;
    lines[middle] = "not json at all";
    let broken = lines.join("\n") + "\n";
    std::fs::write(&segment, &broken).expect("写得进");
    let second = bridge(&home).await;
    let mut napcat = admin_napcat(second.port).await;

    napcat.admin_says(2, "坏了以后的一句").await;
    assert_eq!(napcat.reply().await, "二", "另开一个接着回");
    // 会话的文件夹数：坏了的那个读不出事件，照文件夹数。
    let folders = || {
        let all = segment
            .parent()
            .and_then(|dir| dir.parent())
            .expect("有上一层");
        std::fs::read_dir(all).expect("列得出").count()
    };
    assert_eq!(folders(), 2, "新开了一个，旧的留着");
    assert_eq!(
        std::fs::read_to_string(&segment).expect("读得了"),
        broken,
        "旧的日志一个字不动"
    );
    // 之后照常找回的是新的：不再开第三个。
    napcat.admin_says(3, "再一句").await;
    assert_eq!(napcat.reply().await, "三");
    assert_eq!(folders(), 2);
    assert_eq!(home.said_texts().len(), 2, "两句都说进了新的那个");
    second.stop().await.expect("停得下");
}

#[tokio::test]
async fn the_same_message_number_with_other_words_is_another_message() {
    let script = Script::new([Play::Says("一"), Play::Says("二")]);
    let home = Home::new(&script);
    let bridge = bridge(&home).await;
    let mut napcat = admin_napcat(bridge.port).await;
    napcat.admin_says(7, "头一句").await;
    assert_eq!(napcat.reply().await, "一");
    // 平台的消息编号撞了（同一个编号、同一个时刻，话不一样）：原来核心照编号当成重发，这一句悄悄没了。
    napcat.admin_says(7, "另一句").await;
    assert_eq!(napcat.reply().await, "二");
    assert_eq!(home.said_texts(), ["头一句", "另一句"]);
    bridge.stop().await.expect("停得下");
}
