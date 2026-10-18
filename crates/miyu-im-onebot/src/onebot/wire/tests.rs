//! OneBot 的一条连接（施工 I-2 上）：被顶掉时在等的当场失败、写一帧关闭；取东西照给的时限等，比一次调用久。钟是停住的
//! （`start_paused`）。

use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use super::Wire;
use miyu_im::chat::{CallError, Fetch, Link};

/// 一次调用最多等 10 秒。
const TIMEOUT: Duration = Duration::from_secs(10);

/// 写出去的一帧动作。
async fn frame(frames: &mut mpsc::Receiver<Message>) -> Value {
    match frames.recv().await {
        Some(Message::Text(text)) => serde_json::from_str(&text).expect("是 JSON"),
        other => panic!("不是一帧动作：{other:?}"),
    }
}

#[tokio::test(start_paused = true)]
async fn closing_fails_what_waits_and_tells_the_peer() {
    let (out, mut frames) = mpsc::channel(8);
    let wire = Wire::new(out, TIMEOUT);
    let pending = wire
        .calls
        .begin(&wire.out, "get_msg", json!({}))
        .await
        .expect("放进去了");
    let _ = frame(&mut frames).await;
    wire.close();
    assert_eq!(
        pending.wait().await,
        Err(CallError::Closed),
        "当场失败，不等到时限"
    );
    assert!(
        matches!(frames.recv().await, Some(Message::Close(_))),
        "写了一帧关闭"
    );
    assert_eq!(
        wire.recall(miyu_im::chat::Msg::Number(1)).await,
        Err(CallError::Closed)
    );
}

#[tokio::test(start_paused = true)]
async fn fetching_waits_as_long_as_it_was_given() {
    let (out, mut frames) = mpsc::channel(8);
    let wire = std::sync::Arc::new(Wire::new(out, TIMEOUT));
    let fetching = tokio::spawn({
        let wire = std::sync::Arc::clone(&wire);
        async move { wire.fetch(Fetch::File, "f", Duration::from_secs(60)).await }
    });
    let sent = frame(&mut frames).await;
    assert_eq!(sent["action"], "get_file");
    tokio::time::sleep(Duration::from_secs(30)).await;
    let data = json!({"file_name": " a.mp4 ", "base64": "base64://AA=="});
    assert!(
        wire.calls
            .answer(json!({"status": "ok", "data": data, "echo": sent["echo"]}))
    );
    let located = fetching
        .await
        .expect("没崩")
        .expect("一次调用的时限过了也还在等");
    assert_eq!(located.name.as_deref(), Some("a.mp4"));
}
