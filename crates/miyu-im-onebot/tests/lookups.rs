//! 查询信息（施工 I-44，I-91 把群聊记录、群资料并成一个）：真核心拉起真桥、假 NapCat。插件出厂开着，群里第一轮的工具表里八件都在；
//! 卡片上关掉它，之后的回合里八件都没有；卡片恢复默认、总开关关掉，im 重新登记，之后的回合里八件也没有。

use serde_json::json;

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 查询信息的八件，照名字排。
const LOOKUPS: [&str; 8] = [
    "qq_activity_ranking",
    "qq_avatar",
    "qq_chat_history",
    "qq_contacts",
    "qq_fetch_media",
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
async fn the_lookup_plugin_switches_its_tools() {
    let lines = Lines::new([Line::says("好。"), Line::says("好。"), Line::says("好。")]);
    let (home, napcat) = up(&lines).await;
    called_once(&home, &napcat, 801, 1).await;
    assert_eq!(offered(&lines, 0, &LOOKUPS), LOOKUPS, "出厂开着");

    // 卡片上关掉查询信息（场所的插件开关，核心 O-60）：之后的回合里八件都没有。
    let mut head = Head::connect(&home.root).await;
    let off = json!({"card": {"kind": "other_groups"}, "values": {"plugins": {"miyu-im-qq-lookup": false}}});
    let saved = head.page_call_with("cards.save_one", off).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    called_once(&home, &napcat, 802, 2).await;
    assert!(offered(&lines, 1, &LOOKUPS).is_empty(), "这个群关了");

    // 卡片恢复默认、总开关关掉：im 重新登记，之后的回合里八件也没有。
    let back = json!({"card": {"kind": "other_groups"}, "values": {"plugins": null}});
    let saved = head.page_call_with("cards.save_one", back).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let before = provided(&home);
    set_system(&home, json!([{"key": "im.qq.lookup_on", "value": false}])).await;
    until_provided_again(&home, before).await;
    called_once(&home, &napcat, 803, 3).await;
    assert!(offered(&lines, 2, &LOOKUPS).is_empty(), "总开关关了");
    stopped(home).await;
}

#[tokio::test]
async fn group_only_plugins_are_off_in_a_private_chat() {
    // 施工 I-51（项目主人 10-27：群管理不该出现在私聊卡片里）：群管理、识人术声明了只在群里用，私聊里 im 告诉核心开着的插件里
    // 没有它们；查询信息没限场所，私聊里照样开着。
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
    assert!(
        told.contains("miyu-im-qq-lookup"),
        "没限场所的照样开着：{told}"
    );
    stopped(home).await;
}
