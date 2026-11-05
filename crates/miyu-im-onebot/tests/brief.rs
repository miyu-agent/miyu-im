//! 说短话（施工 I-139）：真核心照开关拉起真的桥，假 NapCat 发群消息，她照台词说。群里开一轮，请求最后那条消息带出厂的
//! 「群里一条消息通常只有几个字」那一块，排在「你只回这几条」前面；卡片关了（`brief = false`）的群不带。

use std::sync::Arc;

use serde_json::json;

use miyu_kernel::request::Request;

use crate::support::group::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 出厂开着的群。
const ON: i64 = 881;

/// 卡片关了的群。
const OFF: i64 = 882;

/// 叫她才回（不过判官）；[`OFF`] 关了说短话。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\ndiscipline = \"when-called\"\n\n\
    [[rule]]\nmatch = { kind = \"group\", group = [882] }\nbrief = false\n";

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 出厂的那一句，去掉行尾。
fn brief() -> String {
    std::fs::read_to_string(resources().join("packages/miyu-im/data/facts/brief.txt"))
        .expect("读得出")
        .trim_end()
        .to_string()
}

/// 一次请求最后一条消息写成的 JSON。
fn last(request: &Request) -> String {
    serde_json::to_string(request.messages.last().expect("有消息")).expect("写得成")
}

/// 终端管理员在群 `group` 里 @ 她：第 `message` 条。
fn admin_calls(napcat: &Answering, group: i64, message: i64) {
    let words = json!([at(BOT), plain(" 在吗")]);
    napcat.send(group_frame(
        group,
        ADMIN,
        message,
        words,
        ("终端管理员", "o"),
    ));
}

#[tokio::test]
async fn a_group_turn_carries_the_brief_note_first_unless_the_card_turns_it_off() {
    let lines = Lines::new([Line::says("在"), Line::says("嗯")]);
    let (home, mut napcat, _) = started_by(Arc::new(lines.clone()), RULES, MEMBERS).await;
    admin_calls(&napcat, ON, 1);
    assert_eq!(napcat.group_reply(ON).await, "在");
    admin_calls(&napcat, OFF, 2);
    assert_eq!(napcat.group_reply(OFF).await, "嗯");
    let requests = lines.requests();
    assert_eq!(requests.len(), 2, "{requests:#?}");
    let note = serde_json::to_string(&brief()).expect("写得成");
    let note = note.trim_matches('"');
    let on = last(&requests[0]);
    let at = on
        .find(note)
        .unwrap_or_else(|| panic!("开着的群带这一句：{on}"));
    let another = on.find("<another-line>").unwrap_or_else(|| panic!("{on}"));
    assert!(at < another, "说短话排第一块：{on}");
    assert!(!last(&requests[1]).contains(note), "卡片关了的群不带");
    stopped(home).await;
}
