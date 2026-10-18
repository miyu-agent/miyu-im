//! 群名、群号（施工 O-49，`onebot.md` 第一条「群名」）：真核心拉起真桥、假 NapCat（每个群都叫 [`GROUP_NAME`]）。群会话 system
//! 的最后是那一段，名字、号对；群里再来一条不再问、不再放；私聊没有。

use std::sync::Arc;

use serde_json::json;

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::*;

/// 群号。
const GROUP: i64 = 777;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 出厂模板换出来的那一段。
fn title() -> String {
    std::fs::read_to_string(resources().join("packages/im/data/system/group.txt"))
        .expect("读得出")
        .replace("{name}", GROUP_NAME)
        .replace("{id}", &GROUP.to_string())
        .trim_end()
        .to_string()
}

#[tokio::test]
async fn a_group_session_knows_its_name_and_number_and_a_private_chat_does_not() {
    let script = Script::new([Play::Says("在"), Play::Says("嗯"), Play::Says("私聊在")]);
    let (home, mut napcat, _) = started(&script, "", "", MEMBERS).await;
    let call = |id: i64, text: &str| {
        group_frame(
            GROUP,
            ADMIN,
            id,
            json!([at(BOT), plain(&format!(" {text}"))]),
            ("老板", "boss"),
        )
    };
    napcat.send(call(1, "在吗"));
    assert_eq!(napcat.group_reply(GROUP).await, "在");
    napcat.send(call(2, "还在吗"));
    assert_eq!(napcat.group_reply(GROUP).await, "嗯");
    let requests = script.requests();
    let system = &requests[0].1.system;
    assert!(
        system.trim_end().ends_with(&title()),
        "群名那一段在 system 最后：{system}"
    );
    assert_eq!(system.matches("<venue>").count(), 1, "只一段");
    assert_eq!(requests[1].1.system, *system, "第二轮 system 逐字节一样");
    let log = run_log(&home.root);
    assert_eq!(
        log.matches("venue tail set").count(),
        1,
        "一小时里不再放：{log}"
    );
    assert_eq!(
        napcat.group_infos(),
        [GROUP],
        "问的是这个群，一小时里只问一次"
    );
    // 私聊不放。
    napcat.send(private_frame(ADMIN, 3, json!([plain("在吗")])));
    let action = napcat.action().await;
    assert_eq!(action["action"], "send_private_msg", "{action}");
    assert_eq!(action["params"]["message"][0]["data"]["text"], "私聊在");
    let private = &script.requests()[2].1.system;
    assert!(!private.contains("<venue>"), "私聊没有：{private}");
    stopped(home).await;
}

#[tokio::test]
async fn asked_every_time_the_same_name_is_set_once() {
    // `group_info_seconds` 是 0：每条消息都问，名字没变的不再交（核心照快照比，交了也不换）。
    let script = Script::new([Play::Says("在"), Play::Says("嗯")]);
    let tuned = json!({"group_info_seconds": 0});
    let (home, mut napcat, _) = started_tuned(Arc::new(script.clone()), "", &tuned, |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    let call = |id: i64| {
        group_frame(
            GROUP,
            ADMIN,
            id,
            json!([at(BOT), plain(" 在吗")]),
            ("老板", "boss"),
        )
    };
    napcat.send(call(1));
    assert_eq!(napcat.group_reply(GROUP).await, "在");
    napcat.send(call(2));
    assert_eq!(napcat.group_reply(GROUP).await, "嗯");
    let log = run_log(&home.root);
    assert_eq!(log.matches("venue tail set").count(), 1, "{log}");
    assert_eq!(napcat.group_infos(), [GROUP, GROUP], "每条都问了");
    stopped(home).await;
}
