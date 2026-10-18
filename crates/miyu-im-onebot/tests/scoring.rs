//! 判断打分挂接点（施工 O-43 末，`onebot.md` 第一条「挂接点」第 5 条，`extending.md` 第二条）：真核心照开关拉起真桥、判官是本机
//! 回环上的假服务器（`support::judge`），管理员家目录里另装导出 `chat.score` 的测试包。
//!
//! - 判官说不回、包加 1 分：回了；判断的 `score` 写着加了多少、谁加的，`bonus` 里算进去了。问的是场所、平台编号、成立的条件、
//!   判官的五维和两个真假。
//! - 判官说回、包减 1 分：不回了。
//! - 包给 0 分：照判官的，判断不写 `hooked`。只查违规的不问包。

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};
use miyu_store::root::DataRoot;

use crate::support::group::*;
use crate::support::hooked::install;
use crate::support::judge::*;
use crate::support::*;

/// 群号。
const GROUP: i64 = 777;

/// 群里的别人。
const LIN: i64 = 20004;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 系统的场所规则：群都不抽样。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0 }\n";

/// 平台编号是 `message` 的那一条判下来的样子。
fn decided(events: &[Value], message: i64) -> Option<Value> {
    let cause = format!("qq:{BOT}:{message}:{TIME}/decided");
    events
        .iter()
        .find(|event| event["kind"] == "ext.im.chat.decided" && event["cause"] == cause)
        .map(|event| event["body"].clone())
}

/// 数据根里测试包 `id` 记调用的文件。
fn log_of(root: &DataRoot, id: &str) -> std::path::PathBuf {
    root.path().join(format!("{id}.calls"))
}

/// 测试包 `id`：判断打分挂接点答 `bonus`。
fn hook(root: &DataRoot, id: &str, bonus: f64) {
    let record = format!("record:{}", log_of(root, id).display());
    let how = format!("answer:chat.score:{{\"bonus\":{bonus}}}");
    install(root, id, &[record, how], &["chat.score"]);
}

/// 等到挂了 `count` 个包（同 `steering.rs`）。
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

/// 小林 @ 她，第 `message` 条。
fn lin_calls(message: i64) -> Value {
    group_frame(
        GROUP,
        LIN,
        message,
        json!([at(BOT), plain(" 在吗")]),
        ("小林", "lin"),
    )
}

#[tokio::test]
async fn a_package_lifts_a_no_into_a_reply_and_is_named() {
    let server = judge(vec![no("not for her")]).await;
    let script = Script::new([Play::Says("在的")]);
    let (home, mut napcat, _) =
        started_judged_installed(&script, &server, (RULES, ""), MEMBERS, |root| {
            hook(root, "boost", 1.0)
        })
        .await;
    until_hooked(&home, 1).await;
    napcat.send(lin_calls(1));
    assert_eq!(napcat.group_reply(GROUP).await, "在的");
    let venue = format!("qq:group:{GROUP}");
    let events = until_events(&home.root, &venue, |events| decided(events, 1).is_some()).await;
    let lifted = decided(&events, 1).expect("记了");
    assert_eq!(lifted["outcome"], "reply", "{lifted}");
    assert_eq!(
        lifted["score"]["hooked"],
        json!({"bonus": 1.0, "packages": ["boost"]}),
        "{lifted}"
    );
    // 条件的加分（冲她来 0.3）加上包的 1。
    assert_eq!(lifted["score"]["bonus"], json!(1.3));
    let asked = &std::fs::read_to_string(log_of(&home.root, "boost")).expect("记了");
    let asked: Value = serde_json::from_str(asked.lines().next().expect("一次")).expect("JSON");
    assert_eq!(asked["method"], "chat.score");
    assert_eq!(
        asked["params"],
        json!({
            "platform": "qq", "venue": venue, "msg": "1", "conditions": [{"kind": "direct", "bonus": 0.3}],
            "scores": [1.0, 1.0, 1.0, 1.0, 1.0], "should_reply": false, "to_bot": false,
        })
    );
    stopped(home).await;
}

#[tokio::test]
async fn a_package_damps_a_yes_into_silence() {
    let server = judge(vec![yes("for her")]).await;
    let script = Script::new([]);
    let (home, napcat, _) =
        started_judged_installed(&script, &server, (RULES, ""), MEMBERS, |root| {
            hook(root, "damp", -1.0)
        })
        .await;
    until_hooked(&home, 1).await;
    napcat.send(lin_calls(1));
    let venue = format!("qq:group:{GROUP}");
    let events = until_events(&home.root, &venue, |events| decided(events, 1).is_some()).await;
    let damped = decided(&events, 1).expect("记了");
    assert_eq!(damped["outcome"], "record", "{damped}");
    assert_eq!(
        damped["score"]["hooked"],
        json!({"bonus": -1.0, "packages": ["damp"]})
    );
    stopped(home).await;
}

#[tokio::test]
async fn a_zero_is_not_written_and_moderation_only_is_not_asked() {
    let server = judge(vec![
        yes("for her"),
        verdict(json!({
            "relevance": 0, "willingness": 0, "social": 0, "timing": 0, "continuity": 0,
            "should_reply": false, "to_bot": false, "severity": 1, "reason": "mild",
        })),
    ])
    .await;
    let script = Script::new([Play::Says("在的")]);
    let (home, mut napcat, _) =
        started_judged_installed(&script, &server, (RULES, "坏词\n"), MEMBERS, |root| {
            hook(root, "zero", 0.0)
        })
        .await;
    until_hooked(&home, 1).await;
    napcat.send(lin_calls(1));
    assert_eq!(napcat.group_reply(GROUP).await, "在的");
    let venue = format!("qq:group:{GROUP}");
    let events = until_events(&home.root, &venue, |events| decided(events, 1).is_some()).await;
    let first = decided(&events, 1).expect("记了");
    assert!(
        first["score"].get("hooked").is_none(),
        "给 0 的不写：{first}"
    );
    let asked = || {
        std::fs::read_to_string(log_of(&home.root, "zero"))
            .unwrap_or_default()
            .lines()
            .count()
    };
    assert_eq!(asked(), 1);
    // 只有违规旗（她没说过话的另一个群、正文里有违规词）：只查违规，不问包。
    let quiet = GROUP + 1;
    napcat.send(group_frame(
        quiet,
        20009,
        2,
        json!([plain("这里有坏词")]),
        ("小乙", "yi"),
    ));
    let quiet_venue = format!("qq:group:{quiet}");
    let events = until_events(&home.root, &quiet_venue, |events| {
        decided(events, 2).is_some()
    })
    .await;
    let checked = decided(&events, 2).expect("记了");
    assert_eq!(checked["judge"]["mode"], "moderation_only", "{checked}");
    assert_eq!(asked(), 1, "只查违规的不问包");
    stopped(home).await;
}
