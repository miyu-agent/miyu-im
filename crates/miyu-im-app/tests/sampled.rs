//! 概率触发主动回复（施工 I-197，设计 12 第一节第 1 条）：真核心照开关拉起真的桥，假 NapCat 发群消息，她照台词说。主动回复判断
//! 关着（叫她才回）、概率触发开着的群，没叫她的消息抽中了就开一轮，请求最后那条消息带 `<sampled>`（这一条没叫她、接不接话她
//! 定）；被叫到的那一轮不带；没开概率触发的群没叫她的只记下。抽不抽得中照千分比写满、写零定，不靠运气。

use std::sync::Arc;

use serde_json::json;

use miyu_kernel::request::Request;

use crate::support::group::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 开着概率触发（千分之一千：每条都中）的群。
const DRAWN: i64 = 891;

/// 没开的群。
const PLAIN: i64 = 892;

/// 群里的别人。
const LIN: i64 = 20005;

/// 叫她才回（不过判官）；[`DRAWN`] 开了概率触发。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\ndiscipline = \"when-called\"\n\n\
    [[rule]]\nmatch = { kind = \"group\", group = [891] }\nchatty = { sample = true, sample_rate = 1000 }\n";

/// 假 NapCat 认得的群成员。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 一次请求最后一条消息写成的 JSON。
fn last(request: &Request) -> String {
    serde_json::to_string(request.messages.last().expect("有消息")).expect("写得成")
}

#[tokio::test]
async fn a_drawn_message_opens_a_turn_that_says_nobody_called_her() {
    let lines = Lines::new([Line::says("我也觉得"), Line::says("在")]);
    let (home, mut napcat, _) = started_by(Arc::new(lines.clone()), RULES, MEMBERS).await;
    // 没开的群：没叫她的只记下。
    napcat.send(group_frame(
        PLAIN,
        LIN,
        1,
        json!([plain("今天真冷")]),
        ("小林", "lin"),
    ));
    // 开着的群：没叫她的抽中了，开一轮。
    napcat.send(group_frame(
        DRAWN,
        LIN,
        2,
        json!([plain("今天真冷")]),
        ("小林", "lin"),
    ));
    assert_eq!(napcat.group_reply(DRAWN).await, "我也觉得");
    // 被叫到的那一轮不带那一句。
    let words = json!([at(BOT), plain(" 在吗")]);
    napcat.send(group_frame(DRAWN, LIN, 3, words, ("小林", "lin")));
    assert_eq!(napcat.group_reply(DRAWN).await, "在");
    let requests = lines.requests();
    assert_eq!(requests.len(), 2, "没开的群没请求模型：{requests:#?}");
    let drawn = last(&requests[0]);
    assert!(
        drawn.contains("<sampled>Nobody called you."),
        "抽中的那一轮说明没人叫她：{drawn}"
    );
    assert!(
        !last(&requests[1]).contains("<sampled>"),
        "被叫到的不带：{}",
        last(&requests[1])
    );
    stopped(home).await;
}
