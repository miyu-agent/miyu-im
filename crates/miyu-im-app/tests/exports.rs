//! 给别的包调的方法（施工 O-42，`onebot.md` 第一条「给别的包调的方法」，`extending.md`）：真核心照开关拉起真桥，头照别的包
//! 的样子经核心的 `package.call` 调（核心只多查一道 `[export]`，那一道由清单测试守着）。
//!
//! - `chat.action`：NapCat 没连着的 `not_connected`；连着的原样转过去（动作名、参数），NapCat 回什么交回什么（成了的、失败的），
//!   不带桥编的 `echo`；`bot` 写了连着的号照它，写了没连着的号 `not_connected`；写错的 `bad_params`；NapCat 不回的
//!   `unanswered`。运行日志记谁调了什么动作，不记参数。
//! - `chat.venue`：群会话照回场所和机器人号，不在回的没有叫她的那一条，带开着的插件（卡片上关掉的不在里面），认不得的 `null`。

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::ports::on_free_port;
use crate::support::spawning::{bridge_up, cli, ports_config, text};
use crate::support::*;

/// 参数里放一个运行日志里不该出现的数。
const SECRET_GROUP: i64 = 424_242;

/// 起一个照开关拉起扩展的核心（`bridge.json` 照 `tuned` 改），打开桥，等它听上：交回核心和 NapCat 的端口。
async fn up(tuned: &Value) -> (Home, u16) {
    on_free_port(async |listen| {
        let home = Home::spawning_tuned(Arc::new(Script::new([])), &ports_config(listen), tuned);
        let started = cli(&home.root, &["start"]).await;
        assert_eq!(started.status.code(), Some(0), "{}", text(&started.stderr));
        bridge_up(&home.root, listen).await?;
        Ok((home, listen))
    })
    .await
}

/// 经 `package.call` 调接入通讯平台 的 `method`：交回整条回应。
async fn exported(head: &mut Head, method: &str, params: Value) -> Value {
    head.call(
        "package.call",
        json!({"package": "miyu-im", "method": method, "params": params}),
    )
    .await
}

/// 桥拒的原因：核心转回来的拒绝里的 `data.message`（桥写的 `message`）；`data.code` 照原因是写错的 -32602、别的 -32010。
fn refused(reply: &Value) -> &str {
    let why = reply["error"]["data"]["message"]
        .as_str()
        .unwrap_or_else(|| panic!("不是拒绝：{reply}"));
    let code = if why == "bad_params" { -32602 } else { -32010 };
    assert_eq!(reply["error"]["data"]["code"], code, "{reply}");
    why
}

/// 等到桥登记了方法：问一个认不得的会话，回 `{"venue": null}`。
async fn registered(head: &mut Head) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        let reply = exported(head, "chat.venue", json!({"session": "nope"})).await;
        if reply["result"] == json!({"venue": null}) {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "等不到登记：{reply}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn actions_go_to_napcat_and_come_back_as_napcat_said() {
    let (home, listen) = up(&Value::Null).await;
    let mut head = Head::connect(&home.root).await;
    registered(&mut head).await;
    let list = json!({"action": "get_group_member_list", "params": {"group_id": SECRET_GROUP}});
    let early = exported(&mut head, "chat.action", list.clone()).await;
    assert_eq!(refused(&early), "not_connected", "{early}");

    let mut napcat = admin_napcat(listen).await;
    napcat.version().await;
    let (reply, action) = tokio::join!(
        exported(&mut head, "chat.action", list.clone()),
        napcat.action()
    );
    assert_eq!(
        (&action["action"], &action["params"]),
        (
            &json!("get_group_member_list"),
            &json!({"group_id": SECRET_GROUP})
        )
    );
    assert_eq!(
        reply["result"],
        json!({"status": "ok", "retcode": 0, "data": {"message_id": 1}, "message": "", "wording": ""}),
        "原样交回，没有 echo"
    );
    // 写的动作、写了连着的号：照样转过去；不写参数的是空对象。
    let ban = json!({"action": "set_group_ban", "params": {"group_id": 1, "user_id": 2, "duration": 60}, "bot": BOT});
    let (reply, action) = tokio::join!(exported(&mut head, "chat.action", ban), napcat.action());
    assert_eq!(action["action"], "set_group_ban");
    assert_eq!(reply["result"]["status"], "ok", "{reply}");
    let (_, action) = tokio::join!(
        exported(
            &mut head,
            "chat.action",
            json!({"action": "_get_model_show"})
        ),
        napcat.action()
    );
    assert_eq!(action["params"], json!({}));
    // NapCat 回了失败：也是回答，原样交回。
    let fetched = tokio::select! {
        reply = exported(&mut head, "chat.action", json!({"action": "get_msg", "params": {"message_id": 7}})) => reply,
        () = async { napcat.action().await; } => unreachable!("取东西的不交出来"),
    };
    assert_eq!(fetched["result"]["status"], "failed", "{fetched}");
    // 没连着的号。
    let other = json!({"action": "get_group_list", "bot": BOT + 1});
    assert_eq!(
        refused(&exported(&mut head, "chat.action", other).await),
        "not_connected"
    );
    // 写错的。
    for wrong in [
        json!({}),
        json!({"action": ""}),
        json!({"action": "Get_group_list"}),
        json!({"action": "get group"}),
        json!({"action": "a".repeat(65)}),
        json!({"action": 1}),
        json!({"action": "get_group_list", "params": [1]}),
        json!({"action": "get_group_list", "params": "x"}),
        json!({"action": "get_group_list", "bot": "a b"}),
        json!({"action": "get_group_list", "bot": 1.5}),
        // 别的平台的（施工 I-1）：这个驱动只认 `qq`。
        json!({"action": "get_group_list", "platform": "telegram"}),
        json!({"action": "get_group_list", "platform": 1}),
    ] {
        let reply = exported(&mut head, "chat.action", wrong.clone()).await;
        assert_eq!(refused(&reply), "bad_params", "{wrong} {reply}");
    }
    assert_eq!(
        refused(&exported(&mut head, "chat.venue", json!({})).await),
        "bad_params"
    );
    assert_eq!(
        refused(
            &exported(
                &mut head,
                "chat.venue",
                json!({"session": "nope", "platform": "telegram"})
            )
            .await
        ),
        "bad_params"
    );
    assert_eq!(
        exported(
            &mut head,
            "chat.venue",
            json!({"session": "nope", "platform": "qq"})
        )
        .await["result"],
        json!({"venue": null}),
        "写明是 qq 的照答"
    );
    let log = run_log(&home.root);
    // 答了的那一行：谁调、什么动作、NapCat 回的 `retcode`（`from` 是空的：调的是头，别的包调的带包的编号）。
    assert!(
        log.lines().any(|line| line.contains("export action from=")
            && line.contains("action=get_group_member_list")
            && line.contains("retcode=0")),
        "{log}"
    );
    assert!(!log.contains(&SECRET_GROUP.to_string()), "不记参数：{log}");
    napcat.close().await;
    let stopped = cli(&home.root, &["stop"]).await;
    assert_eq!(stopped.status.code(), Some(0), "{}", text(&stopped.stderr));
    home.stop_extensions().await;
}

#[tokio::test]
async fn a_silent_napcat_is_unanswered() {
    let (home, listen) = up(&json!({"call_timeout_seconds": 1})).await;
    let mut head = Head::connect(&home.root).await;
    registered(&mut head).await;
    let mut napcat = admin_napcat(listen).await;
    napcat.version().await;
    // 这个假 NapCat 收了不回。
    let reply = exported(
        &mut head,
        "chat.action",
        json!({"action": "get_group_list"}),
    )
    .await;
    assert_eq!(refused(&reply), "unanswered", "{reply}");
    napcat.close().await;
    let stopped = cli(&home.root, &["stop"]).await;
    assert_eq!(stopped.status.code(), Some(0), "{}", text(&stopped.stderr));
    home.stop_extensions().await;
}

#[tokio::test]
async fn a_group_session_names_its_venue_and_bot() {
    let script = Script::new([]);
    let rules = "[[rule]]\nmatch = { kind = \"group\" }\n";
    let (home, napcat, _) = started(&script, rules, "", &[(BOT, "米尤", "miyu")]).await;
    napcat.send(group_frame(
        777,
        20004,
        1,
        json!([plain("大家好")]),
        ("小林", "lin"),
    ));
    let venue = "qq:30003:group:777";
    until_events(&home.root, venue, |events| {
        events
            .iter()
            .any(|event| event["kind"] == "ext.im.chat.decided")
    })
    .await;
    let (session, _) = venue_session(&home.root, venue).expect("有会话");
    let mut head = Head::connect(&home.root).await;
    let reply = exported(&mut head, "chat.venue", json!({"session": session})).await;
    assert_eq!(
        reply["result"],
        json!({"platform": "qq", "venue": venue, "bot": BOT, "origin": null, "plugins": ["miyu-im-identity", "miyu-im-qq-admin", "miyu-im-qq-lookup"]}),
        "这一轮没在回：没有叫她的那一条（施工 S-1 上）{reply}"
    );
    // 施工 P-1 中：卡片上关掉群管理（「陌生群」卡），交回的插件里就没有它。
    let off = json!({"card": {"kind": "other_groups"}, "values": {"plugins": {"miyu-im-qq-admin": false}}});
    let saved = head.page_call_with("cards.save_one", off).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let reply = exported(&mut head, "chat.venue", json!({"session": session})).await;
    assert_eq!(
        reply["result"]["plugins"],
        json!(["miyu-im-identity", "miyu-im-qq-lookup"]),
        "{reply}"
    );
    let reply = exported(&mut head, "chat.venue", json!({"session": "nope"})).await;
    assert_eq!(reply["result"], json!({"venue": null}));
    stopped(home).await;
}
