//! 防御性测试（2026-10-30 发布前第二轮审查）：NapCat 回桥调的动作，不能排在交给 im 的事件后面。
//!
//! 一条连接上，事件和回应都由同一个读的循环读；事件交给 im 的那一道队有上限（`inbound_queue`，出厂 256），满了读的循环就停在
//! 那里等。im 正忙着（这里用一个不答话的核心替身让它一直忙；真机上是 im 自己在等 NapCat 回 `get_msg`、`get_image`、群名片，
//! 或者一个群刷屏刷满了队），这时 NapCat 回的话桥读不到：桥自己调的动作一律等到超时（`call_timeout_seconds`），发出去的话算发送
//! 失败，几条连接共用一道队的，一个群刷屏拖住所有连接。
//!
//! 这里：NapCat 先发 400 条私聊（队满），再回桥连上就问的 `get_version_info`。桥应当照样读到回应、认出实现（后台页的 `status`）。

use std::time::Duration;

use serde_json::json;

use crate::support::fake_core::fake_core;
use crate::support::*;

/// 比队的上限（出厂 256）多。
const FLOOD: i64 = 400;

/// 回了以后最多等多久桥认出实现：读得到的话一下就到。
const SEEN: Duration = Duration::from_secs(5);

#[tokio::test]
async fn napcat_answers_are_read_while_events_wait_for_im() {
    let (_dir, root) = temp_root();
    let _core = fake_core(&root);
    let (serve, relay) = serve_pushing(root, settings());
    let bridge = start(serve).await;
    let mut napcat = admitted(
        bridge.port,
        "/onebot/v11/ws",
        Auth::Bearer(TOKEN),
        Some(BOT),
    )
    .await;
    for message_id in 1..=FLOOD {
        let text = format!("第 {message_id} 条");
        napcat
            .private(
                STRANGER,
                message_id,
                json!([{"type": "text", "data": {"text": text}}]),
            )
            .await;
    }
    napcat.version().await;
    let deadline = tokio::time::Instant::now() + SEEN;
    let mut asked = 0;
    let seen = loop {
        asked += 1;
        let status = page_status(&relay, &format!("status-{asked}")).await;
        if status["napcat"]["implementation"].is_string() {
            break true;
        }
        if tokio::time::Instant::now() >= deadline {
            break false;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    assert!(
        seen,
        "NapCat 回了 get_version_info，{SEEN:?} 以后桥还没读到：回应排在满了的事件队后面"
    );
    drop(napcat);
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn answers_are_read_even_when_the_connection_buffer_overflows() {
    // 施工 I-95：im 一直忙、这条连接的缓冲（`connection_events`）也满了，再读到的事件丢掉（记一行 `event dropped, im busy`；这里桥
    // 在测试进程里跑，运行日志不落盘，不看那一行），读的循环不停，回应照样读到。
    let (_dir, root) = temp_root();
    let _core = fake_core(&root);
    let (serve, relay) = serve_pushing(root, settings());
    let bridge = start(serve).await;
    let mut napcat = admitted(
        bridge.port,
        "/onebot/v11/ws",
        Auth::Bearer(TOKEN),
        Some(BOT),
    )
    .await;
    let tuning = crate::support::shipped::tuning();
    let flood = i64::try_from(tuning.inbound_queue + tuning.connection_events + 200).expect("不大");
    for message_id in 1..=flood {
        napcat
            .private(
                STRANGER,
                message_id,
                json!([{"type": "text", "data": {"text": "刷"}}]),
            )
            .await;
    }
    napcat.version().await;
    let deadline = tokio::time::Instant::now() + SEEN;
    let mut asked = 0;
    loop {
        asked += 1;
        let status = page_status(&relay, &format!("status-{asked}")).await;
        if status["napcat"]["implementation"].is_string() {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "丢了事件以后回应也照样读到"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    drop(napcat);
    bridge.stop().await.expect("停得下");
}
