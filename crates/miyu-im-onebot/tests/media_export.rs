//! `chat.media`（施工 I-39，表情包包偷图）：别的包经核心的 `package.call` 照会话、消息编号、第几样取一张图和它的来历。真核心拉起
//! 真桥、假 NapCat（同 `media.rs`）。
//!
//! - 群里一条带图的消息：交回图的字节（base64）、种类，来历照 im 的聊天记录写场所、编号、谁发的（号和名字）、什么时候。
//! - 拒：没有第几样 `no_item`，不是图的（小黄脸）`not_image`，QQ 说没有这一条的 `not_found`，认不得的会话 `unknown_session`，
//!   参数写错的 `bad_params`。

use serde_json::{Value, json};

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::media::*;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};

/// 经 `package.call` 调 `chat.media`：交回整条回应。
async fn media(head: &mut Head, params: Value) -> Value {
    head.call(
        "package.call",
        json!({"package": "im", "method": "chat.media", "params": params}),
    )
    .await
}

/// 桥拒的原因（核心转回来的 `data.message`）。
fn refused(reply: &Value) -> &str {
    reply["error"]["data"]["message"]
        .as_str()
        .unwrap_or_else(|| panic!("不是拒绝：{reply}"))
}

#[tokio::test]
async fn another_package_gets_a_picture_with_where_it_came_from() {
    let picture = png(5, 5);
    let (home, napcat) = up(&Lines::new([Line::says("嗯。")])).await;
    let words = json!([image("p.jpg", 4096), face("14", "/微笑")]);
    napcat.stock_message(601, in_group(words.clone()));
    napcat.stock_file("p.jpg", Ok(json!({"base64": encoded(&picture)})));
    says(&home, &napcat, (MEMBER, "member"), 601, words).await;
    let (session, _) = venue_session(&home.root, &venue()).expect("有会话");
    let mut head = Head::connect(&home.root).await;
    let reply = media(&mut head, json!({"session": session, "msg": "601"})).await;
    let got = &reply["result"];
    assert_eq!(got["kind"], "image", "{reply}");
    assert_eq!(got["data"], encoded(&picture));
    let origin = &got["origin"];
    assert_eq!(
        (&origin["platform"], &origin["venue"], &origin["msg"]),
        (&json!("qq"), &json!(venue()), &json!("601")),
        "{origin}"
    );
    assert_eq!(
        origin["sender"],
        json!({"id": format!("qq:{MEMBER}"), "name": "小明"}),
        "照聊天记录：谁发的、那时叫什么"
    );
    assert!(origin["at"].as_i64().is_some_and(|at| at > 0), "{origin}");
    let wrong = [
        (
            json!({"session": session, "msg": 601, "index": 3}),
            "no_item",
        ),
        (
            json!({"session": session, "msg": "601", "index": 2}),
            "not_image",
        ),
        (json!({"session": session, "msg": "999"}), "not_found"),
        (json!({"session": "nope", "msg": "601"}), "unknown_session"),
        (json!({"session": session}), "bad_params"),
    ];
    for (params, why) in wrong {
        let reply = media(&mut head, params.clone()).await;
        assert_eq!(refused(&reply), why, "{params}");
    }
    stopped(home).await;
}
