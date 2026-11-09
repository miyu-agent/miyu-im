//! 群里的命令回执几秒后撤回（施工 O-25 上，`onebot.md` 第一条「斜杠命令」第 7 条；18 第十节）：真核心拉起真桥、假 NapCat。
//! 撤不撤照配置项 `im.recall_receipts`（施工 O-47：出厂只撤 `clear`、`clear-memory`），被拒的那一句照旧撤，发出去
//! `receipt_recall_seconds`（出厂 3 秒）以后 `delete_msg` 撤回，撤的是 NapCat 回的那个编号；发命令的那条消息不撤；私聊的回执不撤；
//! 别人被拒什么都不发。

use std::collections::BTreeSet;

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::group::*;
use crate::support::*;

/// 群号。
const GROUP: i64 = 888;

/// 群里的别人：用不了 `/stop`。
const LIN: i64 = 20002;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 核心照中文写的 `/stop` 的回执（`resources/core/human/zh.json`）。
const STOPPED: &str = "已全部停止。";

/// 一个 `send_group_msg`、`send_private_msg` 动作里的字（只有一个文字段）。
fn text_of(action: &Value) -> &str {
    let message = action["params"]["message"].as_array().expect("段的数组");
    assert_eq!(message.len(), 1, "{action}");
    message[0]["data"]["text"].as_str().expect("有字")
}

/// 核心照中文写的 `/clear` 的回执、上下文空着时被拒的那一句。
const CLEARED: &str = "已清空上下文。";
const NOTHING_TO_CLEAR: &str = "上下文为空。";

/// 群里的 `frame`（终端管理员 `ADMIN` 或者小林 `LIN` 说 `text`，编号 `id`）。
fn group_says(who: i64, id: i64, text: &str) -> Value {
    let name = if who == ADMIN {
        ("终端管理员", "o")
    } else {
        ("小林", "lin")
    };
    group_frame(GROUP, who, id, json!([plain(text)]), name)
}

#[tokio::test]
async fn group_receipts_are_recalled_as_configured_and_private_ones_are_not() {
    let (home, mut napcat, _) = started(&Script::new([]), "", "", MEMBERS).await;
    // 私聊的 `/clear`（空的，被拒）：不撤。
    napcat.send(private_frame(ADMIN, 51, json!([plain("/clear")])));
    let private = napcat.action().await;
    assert_eq!(private["action"], "send_private_msg", "{private}");
    assert_eq!(text_of(&private), NOTHING_TO_CLEAR);
    // 群里终端管理员的 `/stop`：回执照发，不在撤的名单里，不撤。别人的 `/stop` 被拒：什么都不发。
    napcat.send(group_says(ADMIN, 52, "/stop"));
    let stopped_receipt = napcat.action().await;
    assert_eq!(text_of(&stopped_receipt), STOPPED, "{stopped_receipt}");
    napcat.send(group_says(LIN, 53, "/stop"));
    // 小林说一句，上下文就不空了：终端管理员 `/clear` 成了，回执撤；再 `/clear` 被拒，那一句也撤。
    napcat.send(group_says(LIN, 54, "大家好"));
    until_event(&home.root, &format!("qq:30003:group:{GROUP}"), |event| {
        event["kind"] == "message.user"
    })
    .await;
    napcat.send(group_says(ADMIN, 55, "/clear"));
    let cleared = napcat.action().await;
    assert_eq!(
        text_of(&cleared),
        CLEARED,
        "别人被拒的不发，下一个就是这一句：{cleared}"
    );
    napcat.send(group_says(ADMIN, 56, "/clear"));
    let refused = napcat.action().await;
    assert_eq!(text_of(&refused), NOTHING_TO_CLEAR, "{refused}");
    // 两个一样等 3 秒，谁先撤不一定：照编号比。私聊的是 FIRST_SENT，`/stop` 的回执是 +1。
    let mut recalled = BTreeSet::new();
    for _ in 0..2 {
        let params = napcat.recalled().await;
        let id = params["message_id"]
            .as_str()
            .expect("撤的是编号，照原样写成字（施工 I-82）");
        recalled.insert(id.parse::<i64>().expect("NapCat 回的是整数"));
    }
    assert_eq!(
        recalled,
        BTreeSet::from([FIRST_SENT + 2, FIRST_SENT + 3]),
        "撤的是 `/clear` 的回执和被拒的那一句"
    );
    let log = until_log(&home, |log| log.matches("receipt recalled").count() == 2).await;
    assert!(
        napcat.pending_recall().is_none(),
        "`/stop` 的回执、发命令的消息、私聊的都不撤：{log}"
    );
    // 名单改成空的（核心推来 `extension.config`）：`/clear` 成了也不撤。
    let mut core = miyu_client::open::Core::connect_running(&home.root, "test")
        .await
        .expect("连得上核心");
    let changes =
        json!({"layer": "system", "changes": [{"key": "im.recall_receipts", "value": []}]});
    core.call("recall-1", "config.set", changes)
        .await
        .expect("改得了");
    // 推送和下一条消息谁先到不一定：等桥记下换了（Windows 的 CI 上撞出来过）。
    until_log(&home, |log| log.contains("recall receipts changed count=0")).await;
    napcat.send(group_says(LIN, 57, "又说一句"));
    until_event(&home.root, &format!("qq:30003:group:{GROUP}"), |event| {
        event["kind"] == "message.user"
            && event["cause"]
                .as_str()
                .is_some_and(|cause| cause.contains(":57:"))
    })
    .await;
    // 等到那一句回执，再等过撤回的时候看没撤。
    napcat.send(group_says(ADMIN, 58, "/clear"));
    let kept = napcat.action().await;
    assert_eq!(text_of(&kept), CLEARED, "{kept}");
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    assert!(napcat.pending_recall().is_none(), "名单空了不撤");
    assert!(napcat.pending().is_none(), "别人被拒的什么都没发");
    // 回执、被拒的那一句都先入队（施工 O-25 中）。
    let queued = |events: &[Value]| -> Vec<Value> {
        of_kind(events, "ext.im.venues.queued")
            .iter()
            .map(|one| one["body"].clone())
            .collect()
    };
    let group = queued(&venue_events(
        &home.root,
        &format!("qq:30003:group:{GROUP}"),
    ));
    assert_eq!(
        group,
        [
            json!({"kind": "receipt", "text": STOPPED}),
            json!({"kind": "receipt", "text": CLEARED}),
            json!({"kind": "receipt", "text": NOTHING_TO_CLEAR}),
            json!({"kind": "receipt", "text": CLEARED}),
        ]
    );
    stopped(home).await;
}

/// 等到运行日志合 `wanted`：交回那时的运行日志。
async fn until_log(home: &Home, wanted: impl Fn(&str) -> bool) -> String {
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        let log = run_log(&home.root);
        if wanted(&log) {
            return log;
        }
        assert!(tokio::time::Instant::now() < deadline, "等不到：{log}");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}
