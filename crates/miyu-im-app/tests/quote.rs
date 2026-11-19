//! 引用指定的一条（施工 I-31）：真核心拉起真桥、假 NapCat、她照台词调 `qq_quote`。这个群的那一条：她这一轮下一句话第一段引用它，
//! @ 发那一条的人、不再自动 @ 叫她的人（施工 I-258，同 QQ 客户端）；下一轮不带。引用她自己的不 @，同一轮也 `qq_mention` 了那个人的
//! 只 @ 一次。别的群的、找不到的编号：不记，答找不到。

use serde_json::{Value, json};

use crate::support::group::*;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 一条发出去的消息里引用的编号、@ 的号。
fn aimed(message: &[Value]) -> (Option<String>, Vec<String>) {
    let reply = message
        .iter()
        .find(|segment| segment["type"] == "reply")
        .and_then(|segment| segment["data"]["id"].as_str().map(str::to_string));
    let ats = message
        .iter()
        .filter(|segment| segment["type"] == "at")
        .filter_map(|segment| segment["data"]["qq"].as_str().map(str::to_string))
        .collect();
    (reply, ats)
}

/// 下一个发进群里的消息（平台动作跳过）。
async fn next_message(napcat: &mut Answering) -> Vec<Value> {
    loop {
        let action = napcat.action().await;
        if action["action"] == "send_group_msg" {
            return action["params"]["message"]
                .as_array()
                .cloned()
                .unwrap_or_default();
        }
    }
}

#[tokio::test]
async fn her_next_words_quote_the_message_she_picked() {
    let lines = Lines::new([
        Line::uses("", "qq_quote", r#"{"msg":"555"}"#),
        Line::says("说的就是这条"),
        Line::says("第二轮"),
        Line::uses("", "qq_quote", r#"{"msg":"666"}"#),
        Line::says("不带"),
    ]);
    let (home, mut napcat) = up(&lines).await;
    napcat.stock_message(
        555,
        json!({"message_type": "group", "group_id": GROUP, "user_id": MEMBER, "message": []}),
    );
    napcat.stock_message(
        666,
        json!({"message_type": "group", "group_id": GROUP + 1, "user_id": MEMBER, "message": []}),
    );
    // 终端管理员 @ 她：她引用 555，那一句 @ 发 555 的人，不再自动 @ 叫她的人。
    let words = json!([at(BOT), plain(" 你看刚才那条")]);
    says(&home, &napcat, (ADMIN, "member"), 101, words).await;
    let first = next_message(&mut napcat).await;
    assert_eq!(
        aimed(&first),
        (Some("555".to_string()), vec![MEMBER.to_string()]),
        "{first:?}"
    );
    assert_eq!(first.last().expect("有字")["data"]["text"], "说的就是这条");
    assert_eq!(ended(&home, 1).await, done("quoted", &[]));
    // 下一轮不带。
    let words = json!([at(BOT), plain(" 在吗")]);
    says(&home, &napcat, (ADMIN, "member"), 102, words).await;
    let second = next_message(&mut napcat).await;
    assert_ne!(aimed(&second).0.as_deref(), Some("555"), "{second:?}");
    // 别的群的那一条：不记，答找不到；她的话不引用它。
    let words = json!([at(BOT), plain(" 再看看")]);
    says(&home, &napcat, (ADMIN, "member"), 103, words).await;
    let third = next_message(&mut napcat).await;
    assert_ne!(aimed(&third).0.as_deref(), Some("666"), "{third:?}");
    assert_eq!(ended(&home, 3).await, refused("not-found", &[]));
    stopped(home).await;
}

#[tokio::test]
async fn quoting_her_own_words_mentions_nobody_and_a_mentioned_author_only_once() {
    let lines = Lines::new([
        Line::uses("", "qq_quote", r#"{"msg":"777"}"#),
        Line::says("我刚才说的"),
        Line::uses("", "qq_mention", r#"{"users":[40001]}"#),
        Line::uses("", "qq_quote", r#"{"msg":"555"}"#),
        Line::says("你说的这条"),
    ]);
    assert_eq!(MEMBER, 40001, "台词里 @ 的就是发 555 的人");
    let (home, mut napcat) = up(&lines).await;
    napcat.stock_message(
        777,
        json!({"message_type": "group", "group_id": GROUP, "user_id": BOT, "message": []}),
    );
    napcat.stock_message(
        555,
        json!({"message_type": "group", "group_id": GROUP, "user_id": MEMBER, "message": []}),
    );
    // 引用她自己的那一条：QQ 客户端不 @ 自己，这里也不 @。
    let words = json!([at(BOT), plain(" 你刚才说什么")]);
    says(&home, &napcat, (ADMIN, "member"), 101, words).await;
    let first = next_message(&mut napcat).await;
    assert_eq!(
        aimed(&first),
        (Some("777".to_string()), Vec::new()),
        "{first:?}"
    );
    // 同一轮先 @ 了发 555 的人、又引用 555：只 @ 他一次。
    let words = json!([at(BOT), plain(" 再看看")]);
    says(&home, &napcat, (ADMIN, "member"), 102, words).await;
    let second = next_message(&mut napcat).await;
    assert_eq!(
        aimed(&second),
        (Some("555".to_string()), vec![MEMBER.to_string()]),
        "{second:?}"
    );
    stopped(home).await;
}
