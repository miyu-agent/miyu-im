//! 群聊记录、群资料（施工 I-44，设计 06 第 3.7 条）：真核心拉起真桥、假 NapCat。两个插件出厂开着，群里第一轮的工具表里八件都在；
//! 卡片上关掉群聊记录，之后的回合里它的三件没有、群资料的照旧；总开关关掉群资料，im 重新登记，之后的回合里它的五件也没有。

use serde_json::json;

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 群聊记录的三件，照名字排。
const RECORDS: [&str; 3] = ["qq_avatar", "qq_chat_history", "qq_fetch_media"];

/// 群资料的五件，照名字排。
const INFO: [&str; 5] = [
    "qq_activity_ranking",
    "qq_contacts",
    "qq_group_files",
    "qq_members",
    "qq_notices",
];

/// 第 `index` 次请求的工具表里，`wanted` 里的那几件（照名字排）。
fn offered(lines: &Lines, index: usize, wanted: &[&str]) -> Vec<String> {
    let mut names: Vec<String> = lines.requests()[index]
        .tools
        .iter()
        .map(|tool| tool.name.clone())
        .filter(|name| wanted.contains(&name.as_str()))
        .collect();
    names.sort();
    names
}

/// 普通群友叫她一声，等这个群第 `turns` 轮结束。
async fn called_once(home: &Home, napcat: &Answering, msg: i64, turns: usize) {
    says(
        home,
        napcat,
        (MEMBER, "member"),
        msg,
        called(json!([plain("在吗")])),
    )
    .await;
    until_events(&home.root, &venue(), |events| {
        of_kind(events, "turn.ended").len() >= turns
    })
    .await;
}

#[tokio::test]
async fn the_records_and_info_plugins_switch_their_tools() {
    let lines = Lines::new([Line::says("好。"), Line::says("好。"), Line::says("好。")]);
    let (home, napcat) = up(&lines).await;
    called_once(&home, &napcat, 801, 1).await;
    assert_eq!(offered(&lines, 0, &RECORDS), RECORDS, "出厂开着");
    assert_eq!(offered(&lines, 0, &INFO), INFO, "出厂开着");

    // 卡片上关掉群聊记录（场所的插件开关，核心 O-60）：之后的回合里没有它的三件，群资料的照旧。
    let mut head = Head::connect(&home.root).await;
    let off = json!({"card": {"kind": "other_groups"}, "values": {"plugins": {"miyu-im-qq-records": false}}});
    let saved = head.page_call_with("cards.save_one", off).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    called_once(&home, &napcat, 802, 2).await;
    assert!(
        offered(&lines, 1, &RECORDS).is_empty(),
        "这个群关了群聊记录"
    );
    assert_eq!(offered(&lines, 1, &INFO), INFO, "群资料照旧");

    // 总开关关掉群资料：im 重新登记，之后的回合里它的五件也没有。
    let before = provided(&home);
    set_system(&home, json!([{"key": "im.qq.info_on", "value": false}])).await;
    until_provided_again(&home, before).await;
    called_once(&home, &napcat, 803, 3).await;
    assert!(offered(&lines, 2, &INFO).is_empty(), "群资料的总开关关了");
    stopped(home).await;
}

#[tokio::test]
async fn group_only_plugins_are_off_in_a_private_chat() {
    // 施工 I-51（项目主人 10-27：群管理不该出现在私聊卡片里）：群管理、识人术声明了只在群里用，私聊里 im 告诉核心开着的插件里
    // 没有它们；群聊记录、群资料没限场所，私聊里照样开着。
    let script = Script::new([Play::Says("在")]);
    let (home, napcat, _) = started(&script, "", "", &[(BOT, "米尤", "miyu")]).await;
    napcat.send(private_frame(ADMIN, 1, json!([plain("在吗")])));
    let venue = format!("venue=qq:30003:private:{ADMIN}");
    let log = until_logged(&home.root, |log| {
        log.lines()
            .any(|line| line.contains("venue plugins told") && line.contains(&venue))
    })
    .await;
    let told = log
        .lines()
        .find(|line| line.contains("venue plugins told") && line.contains(&venue))
        .expect("说过");
    for off in ["miyu-im-qq-admin", "miyu-im-identity"] {
        assert!(!told.contains(off), "只在群里用的私聊里没有 {off}：{told}");
    }
    for on in ["miyu-im-qq-records", "miyu-im-qq-info"] {
        assert!(told.contains(on), "没限场所的照样开着 {on}：{told}");
    }
    stopped(home).await;
}
