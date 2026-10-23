//! 桥当提供者（施工 O-26，`onebot.md` 第一条「提供者和不说话」第 1、2 条）：进程里的桥经内存里的管道连核心（本机套接字上的
//! 核心不认提供者，`provide` 回 `not_a_provider`，桥照样收发）。桥连上核心头一个发 `provide`；测试照核心反向调用的样子推请求：
//! `tool.call` 照工具答（`skip_reply` 答出厂的那一句，不认识的回 `error: true`，平台工具在不认识的会话里答 `unreachable`），别的
//! 方法回「没有这个方法」，`tool.cancel` 不回。说明、答的话是出厂数据：写坏了、要了别的字段、不在的桥起不来。

use std::time::Duration;

use serde_json::{Value, json};

use miyu_chat::Source;
use miyu_config::problem::Code;
use miyu_session::testkit::{Play, Script};

use crate::rules::{clean, copied_resources};
use crate::support::*;

/// 出厂的资源 `software/onebot/<name>` 原样。
fn shipped(name: &str) -> String {
    std::fs::read_to_string(resources().join("packages/im/data").join(name)).expect("读得出")
}

/// 出厂的 `provide` 参数（施工 T-4 从 im 的 `core/provider/tests.rs` 挪来，`tests/stdio.rs` 拿真的程序登记的比）：驱动交的十四件照
/// 登记的先后，说明、参数照 `onebot/tools/<名字>.json`；`qq_skip_reply`、`qq_moderation_log`、`qq_members` 是 `read`，别的是 `venue`（在场所里做的事）；禁言、
/// 踢人、查管理记录、群成员、@ 人只给群。每件带给人看的名字，照包文件夹 `data/human/<语言>.json` 的 `tools.<名字>.name`，三种语言都有；写了
/// `subject` 的带上 `subject`（施工 S-6 上）。
pub fn shipped_tools() -> Value {
    let read = |name: &str| -> Value { serde_json::from_str(&shipped(name)).expect("是 JSON") };
    let human = |language: &str| read(&format!("human/{language}.json"));
    let entry = |name: &str, access: &str, venues: &[&str]| {
        let spec = read(&format!("onebot/tools/{name}.json"));
        let named = |language: &str| human(language)["tools"][name]["name"].clone();
        let mut entry = json!({
            "name": name,
            "description": spec["description"],
            "input_schema": spec["parameters"],
            "access": access,
            "venues": venues,
            "display_name": {"en": named("en"), "ja": named("ja"), "zh": named("zh")},
        });
        let subject = &human("en")["tools"][name]["subject"];
        if subject.is_string() {
            entry["subject"] = subject.clone();
        }
        entry
    };
    let both: &[&str] = &["private", "group"];
    let group: &[&str] = &["group"];
    json!({"tools": [
        entry("qq_skip_reply", "read", both),
        entry("qq_recall", "venue", both),
        entry("qq_mute", "venue", group),
        entry("qq_kick", "venue", group),
        entry("qq_moderation_log", "read", group),
        entry("qq_poke", "venue", both),
        entry("qq_fetch_media", "venue", both),
        entry("qq_avatar", "venue", both),
        entry("qq_members", "read", group),
        entry("qq_mention", "venue", group),
        entry("qq_notices", "read", group),
        entry("qq_set_card", "venue", group),
        entry("qq_quote", "venue", both),
        entry("qq_send", "venue", both),
        entry("qq_contacts", "read", both),
        entry("qq_group_files", "venue", group),
        entry("qq_react", "venue", group),
        entry("qq_activity_ranking", "read", group),
    ]})
}

/// 照核心反向调用的样子拼一次 `tool.call`：编号 `id`，工具 `tool`。
fn tool_call(id: &str, tool: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tool.call",
        "params": {
            "session": "0190f3a1-0000-7000-8000-000000000001",
            "call_id": "c1",
            "tool": tool,
            "args": {"reason": "not for me"},
            "cwd": "/",
        },
    })
}

/// 等到桥回了编号是 `id` 的回应。
async fn answer(relay: &Relay, id: &str) -> Value {
    within("桥回核心", async {
        loop {
            if let Some(found) = relay.answers().into_iter().find(|one| one["id"] == id) {
                return found;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
}

#[tokio::test]
async fn the_bridge_provides_first_and_answers_the_cores_requests() {
    let home = Home::new(&Script::new([Play::Says("在")]));
    let (serve, relay) = serve_pushing(home.root.clone(), settings());
    let bridge = start(serve).await;
    within("桥问核心", async {
        while relay.asked().is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    assert_eq!(
        relay.asked().first().map(String::as_str),
        Some("provide"),
        "握手以后头一个登记：{:?}",
        relay.asked()
    );
    relay.request(json!({
        "jsonrpc": "2.0",
        "method": "tool.cancel",
        "params": {"session": "0190f3a1-0000-7000-8000-000000000001", "call_id": "c0"},
    }));
    relay.request(tool_call("core-1", "qq_skip_reply"));
    relay.request(tool_call("core-2", "ban_all"));
    relay.request(json!({"jsonrpc": "2.0", "id": "core-3", "method": "tool.peek", "params": {}}));
    assert_eq!(
        answer(&relay, "core-1").await,
        json!({"jsonrpc": "2.0", "id": "core-1", "result": {
            "blocks": [{"type": "text", "text": shipped("onebot/tool-results/skipped.txt")}],
            "error": false,
            "end_turn": true,
        }})
    );
    let unknown = shipped("tool-results/unknown.txt").replace("{name}", "ban_all");
    assert_eq!(
        answer(&relay, "core-2").await,
        json!({"jsonrpc": "2.0", "id": "core-2", "result": {
            "blocks": [{"type": "text", "text": unknown}],
            "error": true,
        }})
    );
    let refused = answer(&relay, "core-3").await;
    assert_eq!(refused["error"]["code"], -32601, "{refused}");
    assert_eq!(
        refused["error"]["data"]["reason"], "unknown_method",
        "{refused}"
    );
    // 平台工具（施工 O-31）交给跟核心的那一头答：桥不认识这个会话，答够不着 QQ。
    relay.request(tool_call("core-4", "qq_recall"));
    assert_eq!(
        answer(&relay, "core-4").await,
        json!({"jsonrpc": "2.0", "id": "core-4", "result": {
            "blocks": [{"type": "text", "text": shipped("onebot/tool-results/unreachable.txt")}],
            "error": true,
        }})
    );
    // `tool.cancel` 不回；桥照样收发。
    let mut napcat = admin_napcat(bridge.port).await;
    napcat.admin_says(1, "在吗").await;
    assert_eq!(napcat.reply().await, "在", "provide 被拒了，话照说");
    assert_eq!(relay.answers().len(), 4, "{:?}", relay.answers());
    bridge.stop().await.expect("停得下");
}

#[test]
fn the_shipped_tools_are_read_and_broken_ones_stop_the_bridge() {
    assert!(crate::support::shipped::factory_at(&package_in(resources())).is_ok());
    let resources = copied_resources();
    let software = resources.join("packages/im/data");
    for (file, text, wanted) in [
        (
            "onebot/tools/qq_skip_reply.json",
            "{\"description\": \"x\"}",
            "parameters",
        ),
        (
            "onebot/tool-results/skipped.txt",
            "Skipped {name}.\n",
            "name",
        ),
        ("tool-results/unknown.txt", "No tool {tool}.\n", "tool"),
        // 施工 O-31：平台工具（一）的说明和答的话同样是出厂数据。
        (
            "onebot/tools/qq_mute.json",
            "{\"parameters\": {}}",
            "description",
        ),
        (
            "onebot/tool-results/muted.txt",
            "Muted {who} for {seconds}.\n",
            "seconds",
        ),
        ("onebot/tool-results/failed.txt", "Failed {why}.\n", "why"),
        // 施工 T-1：驱动交的工具照它的目录读，写坏了、不在的同样是打包的错。
        (
            "onebot/tools/qq_members.json",
            "{\"parameters\": {}}",
            "description",
        ),
        (
            "onebot/tool-results/members.txt",
            "{shown} of {all}\n",
            "all",
        ),
        // 施工 Q-3：驱动给场所会话记事实的模板也是。
        ("onebot/facts/muted-by.txt", "{who} for {how}.\n", "how"),
        // 施工 Q-7：驱动问模型的提示词模板也是。
        (
            "onebot/prompts/approval-request.txt",
            "{rule} {answer}\n",
            "answer",
        ),
    ] {
        let path = software.join(file);
        let kept = std::fs::read_to_string(&path).expect("读得出");
        std::fs::write(&path, text).expect("写得进");
        let problems =
            crate::support::shipped::factory_at(&package_in(&resources)).expect_err("写坏了");
        assert_eq!(
            problems
                .iter()
                .map(|problem| (problem.code, problem.source, problem.file.as_str()))
                .collect::<Vec<_>>(),
            [(Code::BadFormat, Source::Factory, file)]
        );
        assert!(
            problems[0]
                .why
                .as_deref()
                .is_some_and(|why| why.contains(wanted)),
            "{problems:?}"
        );
        std::fs::remove_file(&path).expect("删得了");
        let problems =
            crate::support::shipped::factory_at(&package_in(&resources)).expect_err("不在");
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!(problems[0].file, file, "{problems:?}");
        std::fs::write(&path, kept).expect("写得进");
    }
    // 施工 S-6 上：给人看的名字那几份读得到却读不懂的也是打包的错；没有的不算（那种语言不带名字）。
    let zh = software.join("human/zh.json");
    let kept = std::fs::read_to_string(&zh).expect("读得出");
    std::fs::write(&zh, "{").expect("写得进");
    let problems =
        crate::support::shipped::factory_at(&package_in(&resources)).expect_err("读不懂");
    assert_eq!(
        problems
            .iter()
            .map(|problem| (problem.code, problem.file.as_str()))
            .collect::<Vec<_>>(),
        [(Code::BadFormat, "human/zh.json")]
    );
    std::fs::remove_file(&zh).expect("删得了");
    assert!(
        crate::support::shipped::factory_at(&package_in(&resources)).is_ok(),
        "没有的不算错"
    );
    std::fs::write(&zh, kept).expect("写得进");
    // 少要字段照收：答的那一句不带工具名也认得出。
    std::fs::write(software.join("tool-results/unknown.txt"), "No such tool.\n").expect("写得进");
    assert!(crate::support::shipped::factory_at(&package_in(&resources)).is_ok());
    clean(resources.parent().expect("有上一级"));
}
