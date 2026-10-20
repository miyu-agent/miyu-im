//! @ 指定的人（施工 O-55，`onebot.md` 第一条「@ 指定的人」）：真核心拉起真桥、假 NapCat、她照台词调 `mention`。都是群成员的：她
//! 这一轮下一句话第一段前面 @ 他们（重的只写一次），再往后的不带；有不是的整次不记；换了一轮的不带。

use serde_json::{Value, json};

use crate::support::group::*;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 一条发出去的消息里 @ 的号，照先后。
fn ats(message: &[Value]) -> Vec<String> {
    message
        .iter()
        .filter(|segment| segment["type"] == "at")
        .filter_map(|segment| segment["data"]["qq"].as_str().map(str::to_string))
        .collect()
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
async fn her_next_words_mention_the_members_she_named() {
    let lines = Lines::new([
        Line::uses("", "qq_mention", r#"{"users":[40001,40005,40001]}"#),
        Line::says("开会了"),
        Line::says("第二轮"),
        Line::uses("", "qq_mention", r#"{"users":[40001,59999]}"#),
        Line::says("不带"),
    ]);
    let (home, mut napcat) = up(&lines).await;
    // 终端管理员 @ 她：她 @ 小明、小杰（重的只写一次），再说一句。
    let words = json!([at(BOT), plain(" 叫一下他们")]);
    says(&home, &napcat, (ADMIN, "member"), 101, words).await;
    let first = next_message(&mut napcat).await;
    assert_eq!(ats(&first), ["40001", "40005"], "{first:?}");
    assert_eq!(first.last().expect("有字")["data"]["text"], "开会了");
    assert_eq!(
        ended(&home, 1).await,
        done("mentioned", &[("who", "小明, 小杰")])
    );
    // 下一轮不带。
    let words = json!([at(BOT), plain(" 在吗")]);
    says(&home, &napcat, (ADMIN, "member"), 102, words).await;
    let second = next_message(&mut napcat).await;
    assert!(
        ats(&second).iter().all(|qq| qq == &ADMIN.to_string()),
        "{second:?}"
    );
    // 59999 不在群里：整次不记，答哪几个不是；她的话照常、不 @ 他们。
    let words = json!([at(BOT), plain(" 再叫一下")]);
    says(&home, &napcat, (ADMIN, "member"), 103, words).await;
    let third = next_message(&mut napcat).await;
    assert!(!ats(&third).contains(&"40001".to_string()), "{third:?}");
    assert_eq!(
        ended(&home, 3).await,
        refused("not-members", &[("who", "59999")])
    );
    stopped(home).await;
}

#[tokio::test]
async fn a_mention_she_did_not_use_does_not_carry_over() {
    // 她 `mention` 了，这一轮又不说话（`skip_reply`）：下一轮的话不带那几个 @。
    let lines = Lines::new([
        Line::uses("", "qq_mention", r#"{"users":[40001]}"#),
        Line::skips(""),
        Line::says("下一轮"),
    ]);
    let (home, mut napcat) = up(&lines).await;
    let words = json!([at(BOT), plain(" 叫一下他")]);
    says(&home, &napcat, (ADMIN, "member"), 101, words).await;
    ended(&home, 1).await;
    let words = json!([at(BOT), plain(" 在吗")]);
    says(&home, &napcat, (ADMIN, "member"), 102, words).await;
    let next = next_message(&mut napcat).await;
    assert!(!ats(&next).contains(&"40001".to_string()), "{next:?}");
    stopped(home).await;
}
