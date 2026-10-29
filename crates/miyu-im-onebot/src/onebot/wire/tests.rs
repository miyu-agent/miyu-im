//! OneBot 的一条连接（施工 I-2 上）：被顶掉时在等的当场失败、写一帧关闭；取东西照给的时限等，比一次调用久。钟是停住的
//! （`start_paused`）。

use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use super::{Wire, avatar_url};
use miyu_im::chat::{CallError, Face, Fetch, Id, Link, To};

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
    let wire = Wire::new(out, TIMEOUT, (String::new(), String::new()));
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
        wire.recall(&To::Group(Id::from(1)), "1").await,
        Err(CallError::Closed)
    );
}

#[tokio::test(start_paused = true)]
async fn fetching_waits_as_long_as_it_was_given() {
    let (out, mut frames) = mpsc::channel(8);
    let wire = std::sync::Arc::new(Wire::new(out, TIMEOUT, (String::new(), String::new())));
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

#[test]
fn qq_avatars_come_from_the_templates() {
    // 施工 I-82：QQ 的头像照地址模板拼，号要是正整数；模板空着的没有。
    let templates = (
        "https://u/{id}.png".to_string(),
        "https://g/{id}.png".to_string(),
    );
    let url = |face| avatar_url(&templates, &face);
    assert_eq!(
        url(Face::User(Id::from(10001))).as_deref(),
        Some("https://u/10001.png")
    );
    assert_eq!(
        url(Face::Group(Id::from(777))).as_deref(),
        Some("https://g/777.png")
    );
    assert_eq!(url(Face::Group(Id::from(-100))), None, "不是正整数");
    assert_eq!(url(Face::User(Id::new("abc").expect("合写法"))), None);
    let empty = (String::new(), String::new());
    assert_eq!(
        avatar_url(&empty, &Face::User(Id::from(1))),
        None,
        "模板空着"
    );
}

#[test]
fn message_ids_napcat_gives_are_read_as_text() {
    // 施工 I-82：NapCat 回的编号数、字都读成字；写回去照原样（`delete_msg`，施工 O-31）。
    use crate::onebot::{delete_msg, message_id};
    assert_eq!(message_id(&json!(77)).as_deref(), Some("77"));
    assert_eq!(message_id(&json!(" 78 ")).as_deref(), Some("78"));
    assert_eq!(message_id(&json!("")), None);
    assert_eq!(message_id(&json!(null)), None);
    assert_eq!(delete_msg("77").1, json!({"message_id": "77"}));
}
