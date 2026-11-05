//! 会话的说话方式（施工 I-139、I-140）：真核心照开关拉起真的桥，假 NapCat 发群消息，她照台词说。卡片上写了「这个会话的说话方式」的群，
//! 开一轮的请求最后那条消息带 `<style-note>写的那段</style-note>`，排在「你只回这几条」前面，写的字照模板转义；没写的群不带。

use std::sync::Arc;

use serde_json::json;

use miyu_kernel::request::Request;

use crate::support::group::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 写了提醒的群。
const NOTED: i64 = 881;

/// 没写的群。
const PLAIN: i64 = 882;

/// 叫她才回（不过判官）；[`NOTED`] 写了提醒，带一个引号，看转义。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\ndiscipline = \"when-called\"\n\n\
    [[rule]]\nmatch = { kind = \"group\", group = [881] }\nstyle_note = \"短点说，像\\\"群友\\\"那样\"\n";

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

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
async fn a_written_note_rides_first_in_its_group_and_nowhere_else() {
    let lines = Lines::new([Line::says("在"), Line::says("嗯")]);
    let (home, mut napcat, _) = started_by(Arc::new(lines.clone()), RULES, MEMBERS).await;
    admin_calls(&napcat, NOTED, 1);
    assert_eq!(napcat.group_reply(NOTED).await, "在");
    admin_calls(&napcat, PLAIN, 2);
    assert_eq!(napcat.group_reply(PLAIN).await, "嗯");
    let requests = lines.requests();
    assert_eq!(requests.len(), 2, "{requests:#?}");
    // 引号照模板转义成 \u0022，再写进 JSON 是 \\u0022。
    let note = "<style-note>短点说，像\\\\u0022群友\\\\u0022那样</style-note>";
    let noted = last(&requests[0]);
    let at = noted
        .find(note)
        .unwrap_or_else(|| panic!("写了提醒的群带这一段：{noted}"));
    let another = noted
        .find("<another-line>")
        .unwrap_or_else(|| panic!("{noted}"));
    assert!(at < another, "会话的说话方式排第一块：{noted}");
    assert!(!last(&requests[1]).contains("<style-note>"), "没写的群不带");
    stopped(home).await;
}
