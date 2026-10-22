//! 模拟真人一条分多条发送（施工 I-26，`onebot.md`「出站队列」）：真核心拉起真桥、假 NapCat、她照台词说。她的一句话照群聊内核的
//! `bubbles` 拆成两条：群里先后两条，引用、@（出站规则二选一）只在第一条，第二条照停顿晚发；配置项关了的一条发完。

use std::time::{Duration, Instant};

use serde_json::{Value, json};

use miyu_chat::bubbles;
use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::*;

/// 群号。
const GROUP: i64 = 3701;

/// 叫她的人。
const A: i64 = 20021;

/// 假 NapCat 认得的群成员。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (A, "阿甲", "a")];

/// 系统的场所规则：叫她才回（不过判官），总引用、马上 @：第一条一定带引用或 @。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\ndiscipline = \"when-called\"\noutbound = { quote_after = 0, mention_after = \"0s\" }\n";

/// 拆成正好两条的一句（照群聊内核挑：拆法以后调了，这里照样挑得到）和拆出来的两条。
fn two() -> (&'static str, Vec<miyu_chat::Bubble>) {
    let candidates = [
        "这个我也遇到过，重启一下就好了",
        "装完记得更新一下，不行的话再说",
        "我刚看了一下，应该是配置写错了",
        "别急慢慢来，换个镜像源试试",
        "图没传上来，我这边看不到",
        "其实没那么复杂，照文档走就行",
        "我翻了一下文档，这个参数早就改名了",
        "不是不敢，是觉得没必要",
    ];
    candidates
        .iter()
        .map(|text| (*text, bubbles(text)))
        .find(|(_, said)| said.len() == 2)
        .expect("候选里有拆成两条的")
}

/// A @ 她说第 `message` 条。
fn calls(napcat: &Answering, message: i64) {
    napcat.send(group_frame(
        GROUP,
        A,
        message,
        json!([at(BOT), plain(" 在吗")]),
        ("阿甲", "a"),
    ));
}

/// 一条消息里的字（文字段拼起来，去掉 @ 后面那个空格）。
fn words(segments: &[Value]) -> String {
    segments
        .iter()
        .filter(|segment| segment["type"] == "text")
        .filter_map(|segment| segment["data"]["text"].as_str())
        .collect::<String>()
        .trim()
        .to_string()
}

/// 一条消息带不带某种段（`reply`、`at`）。
fn has(segments: &[Value], kind: &str) -> bool {
    segments.iter().any(|segment| segment["type"] == kind)
}

#[tokio::test]
async fn a_reply_goes_out_in_two_messages_with_a_pause() {
    let (text, said) = two();
    let script = Script::new([Play::Says(text)]);
    let (home, mut napcat, _) = started(&script, RULES, "", MEMBERS).await;
    calls(&napcat, 1);
    let first = napcat.group_message(GROUP).await;
    let shown = Instant::now();
    let second = napcat.group_message(GROUP).await;
    let waited = shown.elapsed();
    assert_eq!(words(&first), said[0].text, "{first:?}");
    assert!(
        has(&first, "reply") || has(&first, "at"),
        "第一条带引用或 @：{first:?}"
    );
    assert_eq!(words(&second), said[1].text, "{second:?}");
    assert!(
        !has(&second, "reply") && !has(&second, "at"),
        "后面的不带：{second:?}"
    );
    assert!(
        waited + Duration::from_millis(300) >= said[1].pause,
        "照停顿晚发：等了 {waited:?}，该停 {:?}",
        said[1].pause
    );
    stopped(home).await;
}

#[tokio::test]
async fn turned_off_it_is_one_message() {
    let (text, said) = two();
    let script = Script::new([Play::Says(text)]);
    let (home, mut napcat, _) = started(&script, RULES, "humanlike = false\n", MEMBERS).await;
    calls(&napcat, 1);
    let only = napcat.group_message(GROUP).await;
    assert_eq!(words(&only), text, "{only:?}");
    // 开着时第二条该到的时候也没有。
    tokio::time::sleep(said[1].pause + Duration::from_secs(1)).await;
    assert!(napcat.pending().is_none(), "没有第二条");
    stopped(home).await;
}
