//! 群的配置项（施工 O-41，`onebot.md` 第一条「场所规则和出厂数据」第 8 条）：真核心照开关拉起真桥，写错的配置项后台页、命令行
//! 看得到，改对了就没了。群白名单、白名单以外的群 O-58 上挪进场所卡片（`cards.rs`）。

use serde_json::{Value, json};

use miyu_session::testkit::Script;
use miyu_store::root::DataRoot;

use crate::support::group::*;
use crate::support::*;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 系统的场所规则：群都不抽样。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0 }\n";

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

#[tokio::test]
async fn a_misread_setting_is_shown_until_it_is_fixed() {
    // 施工 O-41 补：睡觉时间写错了，后台页的 `status` 带 `misread`，`miyu onebot status` 说一句；改对了两边都没了。
    let script = Script::new([]);
    let (home, _napcat, _) = started(&script, RULES, "sleep = \"abc\"\n", MEMBERS).await;
    let mut head = crate::support::head::Head::connect(&home.root).await;
    let status = head.until("status", |result| result.is_object()).await;
    assert_eq!(status["misread"], json!(["onebot.sleep"]), "{status}");
    // 命令行说清单里的名字（施工 O-41 补二）。
    let said =
        crate::support::spawning::until_status(&home.root, |out| out.contains("睡觉时间")).await;
    assert!(
        said.contains("这几项配置没认出来，照旧用规则文件里的：睡觉时间"),
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
    crate::support::spawning::until_status(&home.root, |out| !out.contains("睡觉时间")).await;
    stopped(home).await;
}
