//! 后台页（施工 O-28 上，`onebot.md` 第一条「后台页」，`package-pages.md`）：真核心照开关拉起真桥，头照网页给后台页转的样子经
//! 核心走一遍：`package.list` 里接入通讯平台 带图标和后台页；`package.call` 调桥登记的 `status`、`connection.token`，答的是桥手里
//! 最新的（NapCat 连上、令牌去掉都跟着变），没登记的核心不转。施工 I-179（核心 BP-1）：后台页由 im 自己的网页服务给，列表带
//! `page_server`，`package.file` 没有目录可读、回 `no_page`。
//! 运行日志记登记成了、取过令牌，不记令牌的值。桥认不出的方法（核心不会转，测试当核心直接发）回 -32601。

use std::time::Duration;

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::head::Head;
use crate::support::ports::on_free_port;
use crate::support::spawning::{bridge_up, cli, ports_config, served_up, text};
use crate::support::*;

#[tokio::test]
async fn the_page_reaches_the_bridge_through_the_core() {
    let script = Script::new([Play::Says("在")]);
    let (home, listen) = on_free_port(async |listen| {
        let home = Home::spawning(&script, &ports_config(listen));
        let started = cli(&home.root, &["start"]).await;
        assert_eq!(started.status.code(), Some(0), "{}", text(&started.stderr));
        bridge_up(&home.root, listen).await?;
        Ok((home, listen))
    })
    .await;
    let mut head = Head::connect(&home.root).await;

    let listed = head.call("package.list", json!({})).await;
    let onebot = listed["result"]["packages"]
        .as_array()
        .unwrap_or_else(|| panic!("{listed}"))
        .iter()
        .find(|one| one["package"] == "miyu-im")
        .cloned()
        .unwrap_or_else(|| panic!("没有 onebot：{listed}"));
    assert_eq!(onebot["icon"], "message-circle", "{onebot}");
    // 作者、上游（核心 F-12，施工 I-4 补）。
    assert_eq!(onebot["author"], "miyu-agent", "{onebot}");
    assert_eq!(
        onebot["upstream"], "https://github.com/miyu-agent/miyu-im",
        "{onebot}"
    );
    assert_eq!(onebot["page"], true, "{onebot}");
    assert_eq!(
        onebot["page_server"], true,
        "后台页由 im 自己的网页服务给：{onebot}"
    );
    let file = head
        .call("package.file", json!({"package": "miyu-im", "path": ""}))
        .await;
    assert_eq!(file["error"]["data"]["reason"], "no_page", "{file}");

    // 桥起来、开好监听以后才登记：等到调得到，页面服务的端口也报上来了（原来只等调得到，偶尔端口还没报上来，`web` 是空的）。
    let mut status = head.until("status", |result| result["web"].is_u64()).await;
    // 页面服务的端口（施工 I-5 上）：测试的配置另挑的那一个。
    let web = status
        .as_object_mut()
        .and_then(|status| status.remove("web"));
    assert!(web.as_ref().is_some_and(Value::is_u64), "{web:?}");
    assert_eq!(
        status,
        json!({
            "napcat": {"connected": false},
            "host": "127.0.0.1",
            "listen": listen,
            "path": "/ws",
            "token": "set",
            "connections": [{"host": "127.0.0.1", "listen": listen, "token": "set"}],
            "platform": "qq",
            "platforms": ["qq", "tg"],
        })
    );
    let mut napcat = admin_napcat(listen).await;
    napcat.version().await;
    let status = head
        .until("status", |result| result["napcat"]["version"].is_string())
        .await;
    assert_eq!(
        status["napcat"],
        json!({
            "connected": true,
            "self_id": BOT.to_string(),
            "implementation": "NapCat.Onebot",
            "version": "4.8.0",
        })
    );
    assert_eq!(
        head.page_call("connection.token").await["result"],
        json!({"token": TOKEN})
    );
    let unknown = head.page_call("apply").await;
    assert_eq!(
        unknown["error"]["data"]["reason"], "unregistered",
        "核心只转桥登记了的：{unknown}"
    );

    // 去掉令牌：核心推给桥，桥手里没有了。
    let unset = head
        .call(
            "config.set",
            json!({"layer": "system", "changes": [{"key": "im.qq.token", "unset": true}]}),
        )
        .await;
    assert!(unset.get("result").is_some(), "{unset}");
    head.until("status", |result| result["token"] == "none")
        .await;
    assert_eq!(
        head.page_call("connection.token").await["result"],
        json!({"token": null})
    );

    let log = std::fs::read_to_string(miyu_im::dirs::log_file(&home.root)).expect("桥写了运行日志");
    assert!(log.contains("methods registered count=18"), "{log}");
    assert_eq!(log.matches("page token read").count(), 2, "{log}");
    assert!(!log.contains(TOKEN), "运行日志不记令牌的值：{log}");
    napcat.close().await;
    home.stop_extensions().await;
}

#[tokio::test]
async fn the_bridge_registers_its_methods_and_refuses_ones_it_does_not_know() {
    let (dir, root) = temp_root();
    let (mut served, listen) =
        on_free_port(async |listen| Ok((served_up(&root, listen).await?, listen))).await;
    // 握手以后先登记工具，再登记方法（不写时限）：后台页的两个、给别的包调的五个（施工 O-42、O-56、I-8 上、I-39）、场所卡片的两个（施工 O-58 上）、
    // 两页的三个（施工 O-58 下）、会话浮窗的概况（施工 I-7 上补）、平台会话页的三个（施工 I-34 上）。
    let registered = within("桥登记方法", async {
        loop {
            let found = served
                .seen
                .lock()
                .expect("没 panic")
                .iter()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .find(|one| one["method"] == "package.methods");
            if let Some(found) = found {
                return found;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    assert_eq!(
        registered["params"],
        json!({"methods": [
            {"name": "status"},
            {"name": "connection.token"},
            {"name": "chat.action"},
            {"name": "chat.venue"},
            {"name": "chat.media"},
            {"name": "chat.send"},
            {"name": "chat.note"},
            {"name": "cards.get"},
            {"name": "cards.save"},
            {"name": "cards.save_one"},
            {"name": "cards.add"},
            {"name": "cards.remove"},
            {"name": "cards.overview"},
            {"name": "sessions.list"},
            {"name": "sessions.delete"},
            {"name": "tasks.list"},
            {"name": "tasks.save"},
            {"name": "tasks.remove"},
        ]})
    );
    for (id, method) in [
        ("core-1", "apply"),
        ("core-2", "status"),
        ("core-3", "connection.token"),
    ] {
        served
            .send(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "method.call",
                "params": {"method": method, "params": {}},
            }))
            .await;
    }
    let refused = served.answer("core-1").await;
    assert_eq!(
        refused["error"],
        json!({"code": -32601, "message": "unknown_method", "data": {"reason": "unknown_method"}})
    );
    // 测试当的核心握手时没交令牌。页面服务的端口（施工 I-5 上）是系统挑的那一个。
    let mut status = served.answer("core-2").await["result"].clone();
    let web = status
        .as_object_mut()
        .and_then(|status| status.remove("web"));
    assert!(web.as_ref().is_some_and(Value::is_u64), "{web:?}");
    assert_eq!(
        status,
        json!({
            "napcat": {"connected": false},
            "host": "127.0.0.1",
            "listen": listen,
            "path": "/ws",
            "token": "none",
            "connections": [{"host": "127.0.0.1", "listen": listen, "token": "none"}],
            "platform": "qq",
            "platforms": ["qq", "tg"],
        })
    );
    assert_eq!(
        served.answer("core-3").await["result"],
        json!({"token": null})
    );
    drop(served.stdin.take());
    let exited = within("桥退出", served.child.wait_with_output())
        .await
        .expect("等得到");
    assert_eq!(exited.status.code(), Some(0), "{}", text(&exited.stderr));
    let log = std::fs::read_to_string(miyu_im::dirs::log_file(&root)).expect("有运行日志");
    assert!(
        log.contains("page method not understood method=apply"),
        "{log}"
    );
    if std::fs::remove_dir_all(&dir).is_err() {
        // 删不掉就留在临时目录里，不影响测试。
    }
}
