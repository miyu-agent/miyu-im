//! 桥当提供者（施工 O-26，`onebot.md` 第一条「提供者和不说话」第 1、2 条）：`provide` 的参数照出厂的说明；核心说的一行分成
//! 哪一种、请求怎么答。

use std::path::Path;
use std::sync::Arc;

use serde_json::{Value, json};

use super::{Heard, heard};
use crate::core::methods::Methods;
use crate::package::Package;
use crate::rules::{Factory, Tools};

/// 这个仓库的包文件夹 `im/`（施工 S-6 中）。
fn package() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../im")
}

/// 出厂的工具。
fn tools() -> Arc<Tools> {
    Factory::load(&Package::at(package()), None)
        .expect("出厂的读得出")
        .tools()
}

/// 出厂的包文件夹的 `data/<name>` 原样。
fn shipped(name: &str) -> String {
    std::fs::read_to_string(package().join("data").join(name)).expect("读得出")
}

/// 核心反向调 `tool`，编号 `id`。
fn tool_call(id: &str, tool: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tool.call", "params": {
        "session": "s1", "call_id": "c1", "tool": tool, "args": {"reason": "x"}, "cwd": "/",
    }})
}

/// 是一条要答的请求：交回回应。
fn asked(heard: Heard) -> Value {
    match heard {
        Heard::Asked(reply) => reply,
        other => panic!("该答：{other:?}"),
    }
}

#[test]
fn without_a_driver_nothing_is_provided_and_every_tool_is_unknown() {
    // 施工 T-4 起工具全在驱动里（`tests/provider.rs`、`tests/stdio.rs` 照出厂的说明查）：不带驱动读的这一份登记的是空的，
    // 「这一轮不说话」也不认识，答 `unknown.txt` 填上那个名字，编号原样。
    assert_eq!(
        tools().provided(&serde_json::Map::new()),
        json!({"tools": []})
    );
    for (id, tool) in [("core-7", "skip_reply"), ("core-8", "ban_all")] {
        let unknown = shipped("tool-results/unknown.txt").replace("{name}", tool);
        assert_eq!(
            asked(heard(&tools(), &Methods::new(), tool_call(id, tool))),
            json!({"jsonrpc": "2.0", "id": id, "result": {
                "blocks": [{"type": "text", "text": unknown}],
                "error": true,
            }})
        );
    }
}

#[test]
fn other_methods_are_unknown() {
    let reply = asked(heard(
        &tools(),
        &Methods::new(),
        json!({"jsonrpc": "2.0", "id": 9, "method": "tool.peek", "params": {}}),
    ));
    assert_eq!(
        reply,
        json!({"jsonrpc": "2.0", "id": 9, "error": {
            "code": -32601, "message": "unknown_method", "data": {"reason": "unknown_method"},
        }})
    );
}

#[test]
fn pushes_and_answers_pass_on_and_cancels_stop_here() {
    for message in [
        json!({"jsonrpc": "2.0", "method": "event", "params": {"session": "s1", "event": {}}}),
        json!({"jsonrpc": "2.0", "method": "extension.config", "params": {"keys": {}}}),
        json!({"jsonrpc": "2.0", "id": "im-1", "result": {}}),
        json!({"jsonrpc": "2.0", "id": "im-2", "error": {"code": -32010}}),
    ] {
        match heard(&tools(), &Methods::new(), message.clone()) {
            Heard::Other(passed) => assert_eq!(passed, message),
            other => panic!("原样交回：{other:?}"),
        }
    }
    let cancel = json!({"jsonrpc": "2.0", "method": "tool.cancel", "params": {"session": "s1", "call_id": "c1"}});
    assert!(matches!(
        heard(&tools(), &Methods::new(), cancel),
        Heard::Cancelled
    ));
}

#[test]
fn a_method_call_is_answered_by_the_methods() {
    let reply = asked(heard(
        &tools(),
        &Methods::new(),
        json!({"jsonrpc": "2.0", "id": "core-9", "method": "method.call", "params": {"method": "status", "params": {}}}),
    ));
    // 状态还没交进来：同不认识的方法（`core/methods.rs`），编号原样。
    assert_eq!(reply["id"], "core-9", "{reply}");
    assert_eq!(reply["error"]["code"], -32601, "{reply}");
}
