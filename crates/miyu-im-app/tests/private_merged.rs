//! 私聊里她正在回的时候又来的几条并进那一轮（施工 I-199 查压测报告 F2 时补的守卫）：这是核心本来就有的行为（场所会话正在跑一轮
//! 时又说进来的话并进去，正在等的那一次模型请求作废、带上新的重来），桥不用另做；这里守着它，核心哪天改了这里先红。F2 里
//! 作废的回复是出站队列的停顿攒过了期限，修在 `route/queue.rs`。

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::oneshot;

use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 全部会话里某一种事件有几条。
fn count(home: &Home, kind: &str) -> usize {
    home.sessions()
        .iter()
        .flat_map(|session| home.events(session))
        .filter(|event| event["kind"] == kind)
        .count()
}

#[tokio::test]
async fn messages_sent_while_she_is_answering_join_that_turn() {
    // 前两次请求都卡着不回（等不到放行）：后来的一条并进来时核心把正在等的那一次作废、带上新的一条重来。
    let (_first, held_first) = oneshot::channel();
    let (_second, held_second) = oneshot::channel();
    let lines = Lines::new([
        Line::says("只看到第一条").released_by(held_first),
        Line::says("只看到前两条").released_by(held_second),
        Line::says("三条一起回"),
    ]);
    let home = Home::speaking(Arc::new(lines.clone()));
    let bridge = bridge(&home).await;
    let mut napcat = admin_napcat(bridge.port).await;
    napcat.admin_says(1, "第一条").await;
    // 等这一轮开了、桥也听到了（照推送记的），再一条一条发。
    within("这一轮开了", async {
        while count(&home, "turn.started") == 0 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    for (number, text) in [(2, "第二条"), (3, "第三条")] {
        napcat.admin_says(number, text).await;
        within("说进去了", async {
            while home.said_texts().len() < usize::try_from(number).expect("正的") {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await;
    }
    assert_eq!(napcat.reply().await, "三条一起回");
    within("这一轮完了", async {
        while count(&home, "turn.ended") == 0 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert_eq!(home.said_texts(), ["第一条", "第二条", "第三条"]);
    assert_eq!(count(&home, "turn.started"), 1, "三条只开了一轮");
    assert_eq!(count(&home, "message.assistant"), 1, "只回一次");
    bridge.stop().await.expect("停得下");
}
