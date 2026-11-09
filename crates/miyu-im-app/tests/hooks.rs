//! 挂接点（施工 O-43 上，`onebot.md` 第一条「挂接点」，`extending.md` 第二条）：真核心照开关拉起真桥，管理员家目录里另装一个
//! 导出 `chat.event` 的测试包（`miyu-im-test-hook`）。NapCat 推来的通知、请求原样交到它那里（带 `from: "im"`），消息不交；
//! 只导出别的方法的包不收。桥起来以后才打开的包，照推来的 `package.changed` 挂上。

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use miyu_session::testkit::Script;
use miyu_store::root::DataRoot;

use crate::support::hooked::{install, install_starting};
use crate::support::ports::on_free_port;
use crate::support::spawning::{bridge_up, cli, ports_config, text};
use crate::support::*;

/// 测试包记下的调用：每行一次。
fn calls(path: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// 等到 `path` 里记下的合 `wanted`：交回那时的全部。最多六十秒。
async fn until_calls(path: &std::path::Path, wanted: impl Fn(&[Value]) -> bool) -> Vec<Value> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        let seen = calls(path);
        if wanted(&seen) {
            return seen;
        }
        assert!(tokio::time::Instant::now() < deadline, "等不到：{seen:#?}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// 数据根 `root` 里测试包 `id` 记调用的文件。
fn log_of(root: &DataRoot, id: &str) -> std::path::PathBuf {
    root.path().join(format!("{id}.calls"))
}

/// 一条群成员进群的通知。
fn joined(user: i64) -> Value {
    json!({"post_type": "notice", "notice_type": "group_increase", "sub_type": "approve", "time": TIME, "self_id": BOT, "group_id": 777, "user_id": user, "operator_id": 0})
}

#[tokio::test]
async fn notices_and_requests_reach_the_package_hooked_on_events() {
    let (home, listen) = on_free_port(async |listen| {
        let home = Home::spawning_with_installed(
            Arc::new(Script::new([])),
            &ports_config(listen),
            |root| {
                let hooky = format!("record:{}", log_of(root, "hooky").display());
                install(
                    root,
                    "hooky",
                    &[hooky, "answer:chat.event:{}".to_string()],
                    &["chat.event"],
                );
                let other = format!("record:{}", log_of(root, "other").display());
                install(
                    root,
                    "other",
                    &[other, "answer:qq.other:{}".to_string()],
                    &["qq.other"],
                );
            },
        );
        let started = cli(&home.root, &["start"]).await;
        assert_eq!(started.status.code(), Some(0), "{}", text(&started.stderr));
        bridge_up(&home.root, listen).await?;
        Ok((home, listen))
    })
    .await;
    let mut napcat = admin_napcat(listen).await;
    napcat.version().await;
    // 测试包可能比桥晚一点跑起来：照推来的 `package.changed` 补上。一直发，直到它收到第一条。
    let hooky = log_of(&home.root, "hooky");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while calls(&hooky).is_empty() {
        assert!(tokio::time::Instant::now() < deadline, "测试包一直没收到");
        napcat.send(joined(5)).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let first = &calls(&hooky)[0];
    assert_eq!(first["method"], "chat.event");
    assert_eq!(first["from"], "miyu-im");
    assert_eq!(
        first["params"],
        json!({"platform": "qq", "bot": BOT, "event": joined(5)}),
        "原样，带上平台"
    );
    // 请求也交；消息不交。
    let request = json!({"post_type": "request", "request_type": "friend", "time": TIME, "self_id": BOT, "user_id": 66, "comment": "hi", "flag": "f-1"});
    napcat.send(request.clone()).await;
    napcat
        .private(
            20009,
            1,
            json!([{"type": "text", "data": {"text": "你好"}}]),
        )
        .await;
    napcat.send(joined(6)).await;
    // 各起一个小任务交、核心也各自转：先后不保证，两条都到了再看。
    let seen = until_calls(&hooky, |seen| {
        let has = |user: i64| {
            seen.iter()
                .any(|call| call["params"]["event"]["user_id"] == user)
        };
        has(6) && has(66)
    })
    .await;
    let events: Vec<&Value> = seen.iter().map(|call| &call["params"]["event"]).collect();
    assert!(events.contains(&&request), "{events:#?}");
    assert!(
        events.iter().all(|event| event["post_type"] != "message"),
        "消息不交：{events:#?}"
    );
    assert!(
        calls(&log_of(&home.root, "other")).is_empty(),
        "只导出别的方法的不收"
    );
    napcat.close().await;
    let stopped = cli(&home.root, &["stop"]).await;
    assert_eq!(stopped.status.code(), Some(0), "{}", text(&stopped.stderr));
    home.stop_extensions().await;
}

#[tokio::test]
async fn a_package_switched_on_after_the_bridge_is_hooked_from_the_pushed_change() {
    let (home, listen) = on_free_port(async |listen| {
        let home = Home::spawning_with_installed(
            Arc::new(Script::new([])),
            &ports_config(listen),
            |root| {
                let late = format!("record:{}", log_of(root, "late").display());
                let args = [late, "answer:chat.event:{}".to_string()];
                install_starting(root, "late", &args, &["chat.event"], "manual");
            },
        );
        let started = cli(&home.root, &["start"]).await;
        assert_eq!(started.status.code(), Some(0), "{}", text(&started.stderr));
        bridge_up(&home.root, listen).await?;
        Ok((home, listen))
    })
    .await;
    let mut napcat = admin_napcat(listen).await;
    napcat.version().await;
    // 桥订阅时它还关着：不算挂上。打开它，核心推 `package.changed`（`status` 变成 `running`），桥照它挂上。
    let mut core = miyu_client::open::Core::connect_running(&home.root, "test")
        .await
        .expect("连得上核心");
    core.call("late", "extension.enable", json!({"package": "late"}))
        .await
        .expect("开得了");
    let late = log_of(&home.root, "late");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while calls(&late).is_empty() {
        assert!(tokio::time::Instant::now() < deadline, "打开以后一直没收到");
        napcat.send(joined(7)).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(calls(&late)[0]["params"]["event"], joined(7));
    napcat.close().await;
    let stopped = cli(&home.root, &["stop"]).await;
    assert_eq!(stopped.status.code(), Some(0), "{}", text(&stopped.stderr));
    home.stop_extensions().await;
}

#[tokio::test]
async fn event_hooks_that_never_answer_are_capped_and_let_go() {
    // 施工 I-96（审查 I2-5）：挂在 `chat.event` 的包收了不答，桥交给它的同时最多 64 件、多的不交；每件最多等 `hook_millis`，
    // 到点让出名额，后来的照样交得到。
    let (home, listen) = on_free_port(async |listen| {
        let home = Home::spawning_with_installed(
            Arc::new(Script::new([])),
            &ports_config(listen),
            |root| {
                let hung = format!("record:{}", log_of(root, "hung").display());
                install(
                    root,
                    "hung",
                    &[hung, "slow:chat.event".to_string()],
                    &["chat.event"],
                );
            },
        );
        let started = cli(&home.root, &["start"]).await;
        assert_eq!(started.status.code(), Some(0), "{}", text(&started.stderr));
        bridge_up(&home.root, listen).await?;
        Ok((home, listen))
    })
    .await;
    let mut napcat = admin_napcat(listen).await;
    napcat.version().await;
    let hung = log_of(&home.root, "hung");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while calls(&hung).is_empty() {
        assert!(tokio::time::Instant::now() < deadline, "测试包一直没收到");
        napcat.send(joined(5)).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    // 先前那几件到点让出名额。
    let wait = Duration::from_millis(crate::support::shipped::tuning().hook_millis);
    tokio::time::sleep(wait * 2).await;
    let before = calls(&hung).len();
    for user in 100..170 {
        napcat.send(joined(user)).await;
    }
    tokio::time::sleep(wait / 2).await;
    let burst = calls(&hung).len() - before;
    assert!(burst <= 64, "收了不答的同时最多交 64 件，交了 {burst} 件");
    tokio::time::sleep(wait * 2).await;
    let settled = calls(&hung).len();
    napcat.send(joined(999)).await;
    until_calls(&hung, |seen| seen.len() > settled).await;
    crate::support::group::stopped(home).await;
}
