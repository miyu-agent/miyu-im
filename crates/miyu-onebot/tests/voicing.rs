//! 出站挂接点（施工 O-43 下，`onebot.md` 第一条「挂接点」第 4 条，`extending.md` 第二条）：真核心照开关拉起真桥，管理员家目录里
//! 另装导出 `qq.outbound` 的测试包。
//!
//! - 换字的：她的话照换成的发（群里、私聊都是）；问的是场所、号、她这一段的原话。
//! - 两个包接力：前一个换了的交给后一个。
//! - 说不发的：不发，运行日志记一行 `reply dropped why=hooked`。
//! - 回得不对的：照原样发、记一行。

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};
use miyu_store::root::DataRoot;

use crate::support::group::*;
use crate::support::hooked::install;
use crate::support::*;

/// 群号。
const GROUP: i64 = 777;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 系统的场所规则：群都不抽样。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0 }\n";

/// 数据根里测试包 `id` 记调用的文件。
fn log_of(root: &DataRoot, id: &str) -> std::path::PathBuf {
    root.path().join(format!("{id}.calls"))
}

/// 测试包 `id`：出站挂接点照 `answer` 答。
fn hook(root: &DataRoot, id: &str, answer: &str) {
    let record = format!("record:{}", log_of(root, id).display());
    let how = format!("answer:qq.outbound:{answer}");
    install(root, id, &[record, how], &["qq.outbound"]);
}

/// 测试包 `id` 记下的调用。
fn calls(root: &DataRoot, id: &str) -> Vec<Value> {
    std::fs::read_to_string(log_of(root, id))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// 等到挂了 `count` 个包（同 `steering.rs`：桥运行日志最后一行说挂了几个的）。
async fn until_hooked(home: &Home, count: usize) {
    let wanted = format!("count={count}");
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        let log = run_log(&home.root);
        let last = log
            .lines()
            .rev()
            .find(|line| line.contains("hooks learned") || line.contains("hooks changed"));
        if last.is_some_and(|line| line.split_whitespace().any(|word| word == wanted)) {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "等不到挂上 {count} 个：{log}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

/// 终端管理员在群里 @ 她，第 `message` 条。
fn admin_calls(message: i64) -> Value {
    group_frame(
        GROUP,
        ADMIN,
        message,
        json!([at(BOT), plain(" 在吗")]),
        ("主人", "admin"),
    )
}

/// 下一个动作是发给终端管理员的私聊：交回里面的字。
async fn private_reply(napcat: &mut Answering) -> String {
    let action = napcat.action().await;
    assert_eq!(action["action"], "send_private_msg", "{action}");
    action["params"]["message"][0]["data"]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[tokio::test]
async fn her_words_go_out_as_the_package_changed_them_in_groups_and_in_private() {
    let script = Script::new([Play::Says("在。"), Play::Says("私聊在。")]);
    let (home, mut napcat, _) = started_installed(&script, RULES, MEMBERS, |root| {
        hook(root, "polish", r#"{"text":"换过的"}"#);
    })
    .await;
    until_hooked(&home, 1).await;
    napcat.send(admin_calls(1));
    assert_eq!(napcat.group_reply(GROUP).await, "换过的");
    let asked = &calls(&home.root, "polish")[0];
    assert_eq!(asked["method"], "qq.outbound");
    assert_eq!(asked["from"], "onebot");
    assert_eq!(
        asked["params"],
        json!({"venue": format!("qq:group:{GROUP}"), "bot": BOT, "text": "在。"})
    );
    napcat.send(private_frame(ADMIN, 2, json!([plain("在吗")])));
    assert_eq!(private_reply(&mut napcat).await, "换过的");
    assert_eq!(calls(&home.root, "polish")[1]["params"]["text"], "私聊在。");
    stopped(home).await;
}

#[tokio::test]
async fn two_packages_pass_the_words_along() {
    let script = Script::new([Play::Says("在。")]);
    let (home, mut napcat, _) = started_installed(&script, RULES, MEMBERS, |root| {
        hook(root, "aaa", r#"{"text":"甲换的"}"#);
        hook(root, "zzz", "{}");
    })
    .await;
    until_hooked(&home, 2).await;
    napcat.send(admin_calls(1));
    assert_eq!(napcat.group_reply(GROUP).await, "甲换的");
    assert_eq!(
        calls(&home.root, "zzz")[0]["params"]["text"],
        "甲换的",
        "后一个拿到的是换过的"
    );
    stopped(home).await;
}

#[tokio::test]
async fn a_dropped_reply_is_not_sent_and_is_logged() {
    let script = Script::new([Play::Says("别发这句")]);
    let (home, napcat, _) = started_installed(&script, RULES, MEMBERS, |root| {
        hook(root, "censor", r#"{"drop":true}"#);
    })
    .await;
    until_hooked(&home, 1).await;
    napcat.send(admin_calls(1));
    let deadline = tokio::time::Instant::now() + WAIT;
    while !run_log(&home.root).contains("why=hooked") {
        assert!(
            tokio::time::Instant::now() < deadline,
            "{}",
            run_log(&home.root)
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let log = run_log(&home.root);
    assert!(
        log.lines().any(|line| line.contains("reply dropped")
            && line.contains("why=hooked")
            && line.contains("package=censor")),
        "{log}"
    );
    assert!(!log.contains("别发这句"), "不记原文");
    let mut napcat = napcat;
    assert!(napcat.pending().is_none(), "什么都不发");
    stopped(home).await;
}

#[tokio::test]
async fn a_wrong_answer_leaves_her_words_as_they_were() {
    let script = Script::new([Play::Says("原样")]);
    let (home, mut napcat, _) = started_installed(&script, RULES, MEMBERS, |root| {
        hook(root, "odd", r#"{"text":""}"#);
    })
    .await;
    until_hooked(&home, 1).await;
    napcat.send(admin_calls(1));
    assert_eq!(napcat.group_reply(GROUP).await, "原样");
    let log = run_log(&home.root);
    assert!(
        log.lines().any(|line| line.contains("hook failed")
            && line.contains("package=odd")
            && line.contains("why=bad_answer")),
        "{log}"
    );
    stopped(home).await;
}

#[tokio::test]
async fn a_silent_hook_is_waited_for_then_her_words_go_as_they_were() {
    let script = Script::new([Play::Says("照旧")]);
    let (home, mut napcat, _) = started_installed(&script, RULES, MEMBERS, |root| {
        let record = format!("record:{}", log_of(root, "slow").display());
        install(
            root,
            "slow",
            &[record, "slow:qq.outbound".to_string()],
            &["qq.outbound"],
        );
    })
    .await;
    until_hooked(&home, 1).await;
    let sent = std::time::Instant::now();
    napcat.send(admin_calls(1));
    assert_eq!(napcat.group_reply(GROUP).await, "照旧");
    assert!(
        sent.elapsed() >= std::time::Duration::from_millis(1500),
        "等满了 hook_millis"
    );
    let log = run_log(&home.root);
    assert!(
        log.lines().any(|line| line.contains("hook failed")
            && line.contains("package=slow")
            && line.contains("why=timeout")),
        "{log}"
    );
    stopped(home).await;
}
