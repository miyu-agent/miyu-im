//! 每一轮末尾的那一句（施工 I-172，核心 RT-1 的 `tail`）：真核心照开关拉起真的桥，假 NapCat 发群消息，她照台词说。群里开的每一轮，
//! 请求带着 `<context-style-note>`（核心放在请求的 `tail`，发出去时接在最末尾）；它不进历史：两轮的消息里都没有它。

use std::sync::Arc;

use serde_json::json;

use miyu_kernel::request::Request;

use crate::support::group::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 群。
const GROUP: i64 = 891;

/// 叫她才回（不过判官）。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\ndiscipline = \"when-called\"\n";

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 尾巴的开头。
const TAIL: &str = "<context-style-note>";

/// 一次请求每一条消息写成的 JSON。
fn messages(request: &Request) -> Vec<String> {
    request
        .messages
        .iter()
        .map(|message| serde_json::to_string(message).expect("写得成"))
        .collect()
}

/// 终端管理员在群里 @ 她：第 `message` 条。
fn admin_calls(napcat: &Answering, message: i64) {
    let words = json!([at(BOT), plain(" 在吗")]);
    napcat.send(group_frame(
        GROUP,
        ADMIN,
        message,
        words,
        ("终端管理员", "o"),
    ));
}

#[tokio::test]
async fn every_group_turn_ends_with_the_note_and_it_stays_out_of_history() {
    let lines = Lines::new([Line::says("在"), Line::says("嗯")]);
    let (home, mut napcat, _) = started_by(Arc::new(lines.clone()), RULES, MEMBERS).await;
    admin_calls(&napcat, 1);
    assert_eq!(napcat.group_reply(GROUP).await, "在");
    admin_calls(&napcat, 2);
    assert_eq!(napcat.group_reply(GROUP).await, "嗯");
    let requests = lines.requests();
    assert_eq!(requests.len(), 2, "{requests:#?}");
    for request in &requests {
        // 核心把它放在请求的 `tail`，各家接口发出去时才接在最末尾（核心 RT-1）。
        let tail = request.tail.as_deref().unwrap_or_default();
        assert!(tail.starts_with(TAIL), "这一轮带尾巴：{tail:?}");
        let all = messages(request);
        assert!(
            all.iter().all(|one| !one.contains(TAIL)),
            "历史、这一轮的消息里都没有：{all:#?}"
        );
    }
    stopped(home).await;
}
