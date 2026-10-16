//! 群的配置项（施工 O-41，`onebot.md` 第一条「场所规则和出厂数据」第 8 条）：真核心照开关拉起真桥，系统配置写群白名单、关掉
//! 白名单以外的群；白名单以外的群里别人 @ 她只记下（`not_allowed`），白名单群照常问判官、回；核心改了配置（`config.set`），
//! 桥不重启，下一条照新的。

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};
use miyu_store::root::DataRoot;

use crate::support::group::*;
use crate::support::*;

/// 群里的别人。
const LIN: i64 = 20004;

/// 白名单里的群。
const LISTED: i64 = 777;

/// 白名单以外的群。
const OTHER: i64 = 666;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 系统的场所规则：群都不抽样。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0 }\n";

/// 平台编号是 `message` 的那一条判下来的样子。
fn decided(events: &[Value], message: i64) -> Option<Value> {
    let cause = format!("qq:{BOT}:{message}:{TIME}/decided");
    events
        .iter()
        .find(|event| event["kind"] == "ext.onebot.chat.decided" && event["cause"] == cause)
        .map(|event| event["body"].clone())
}

/// 别人在群 `group` 里 @ 她，平台编号是 `message`。
fn called(group: i64, message: i64) -> Value {
    group_frame(
        group,
        LIN,
        message,
        json!([at(BOT), plain(" 在吗")]),
        ("小林", "lin"),
    )
}

/// 照网页、命令行的样子改系统配置。
async fn set(root: &DataRoot, changes: Value) {
    let mut core = miyu_client::open::Core::connect_running(root, "test")
        .await
        .expect("连得上核心");
    core.call(
        "o41",
        "config.set",
        json!({"layer": "system", "changes": changes}),
    )
    .await
    .unwrap_or_else(|refused| panic!("config.set 被拒：{refused}"));
}

/// 运行日志里 `venue rules read` 有几行：桥每重读一次场所规则记一行。
fn reads(root: &DataRoot) -> usize {
    run_log(root).matches("venue rules read").count()
}

#[tokio::test]
async fn only_whitelisted_groups_are_answered_until_the_setting_changes() {
    let script = Script::new([Play::Says("在的"), Play::Says("来了")]);
    let onebot = format!("groups = [{LISTED}]\nother_groups = false\n");
    let (home, mut napcat, _) = started(&script, RULES, &onebot, MEMBERS).await;
    let other = format!("qq:group:{OTHER}");
    napcat.send(called(OTHER, 1));
    let events = until_events(&home.root, &other, |events| decided(events, 1).is_some()).await;
    let refused = decided(&events, 1).expect("记了");
    assert_eq!(
        (&refused["inbound"], &refused["why"]),
        (&json!("record_only"), &json!("not_allowed")),
        "{refused}"
    );
    napcat.send(called(LISTED, 2));
    assert_eq!(napcat.group_reply(LISTED).await, "在的");
    // 打开白名单以外的群：桥当场重读，不重启。
    let before = reads(&home.root);
    set(
        &home.root,
        json!([{"key": "onebot.other_groups", "value": true}]),
    )
    .await;
    let deadline = tokio::time::Instant::now() + WAIT;
    while reads(&home.root) == before {
        assert!(
            tokio::time::Instant::now() < deadline,
            "等不到重读：{}",
            run_log(&home.root)
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    napcat.send(called(OTHER, 3));
    assert_eq!(napcat.group_reply(OTHER).await, "来了");
    assert!(napcat.pending().is_none(), "别的什么都不发");
    stopped(home).await;
}

#[tokio::test]
async fn a_misread_setting_is_shown_until_it_is_fixed() {
    // 施工 O-41 补：睡觉时间写错了，后台页的 `status` 带 `misread`，`miyu onebot status` 说一句；改对了两边都没了。
    let script = Script::new([]);
    let (home, _napcat, _) = started(&script, RULES, "sleep = \"abc\"\n", MEMBERS).await;
    let mut head = crate::support::head::Head::connect(&home.root).await;
    let status = head.until("status", |result| result.is_object()).await;
    assert_eq!(status["misread"], json!(["onebot.sleep"]), "{status}");
    let said =
        crate::support::spawning::until_status(&home.root, |out| out.contains("onebot.sleep"))
            .await;
    assert!(
        said.contains("这几项配置没认出来，照旧用规则文件里的：onebot.sleep"),
        "{said}"
    );
    set(
        &home.root,
        json!([{"key": "onebot.sleep", "value": "23:00-07:00"}]),
    )
    .await;
    let status = head
        .until("status", |result| result.get("misread").is_none())
        .await;
    assert_eq!(status["platform"], "qq");
    crate::support::spawning::until_status(&home.root, |out| !out.contains("onebot.sleep")).await;
    stopped(home).await;
}
