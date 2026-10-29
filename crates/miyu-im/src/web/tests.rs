//! `miyu im web` 信不信状态文件写着的端口：im 在跑、进程号对得上才信（状态文件可能是别的进程留下的）。整条路在
//! `tests/web.rs`；桥跑着时状态文件变了就重写，进程号对不上这一条放在这里查（原来在整条路里改文件，Windows 上偶发被桥盖回去）。

use serde_json::json;

use super::page_port;

/// 核心的 `extension.status`：im 的状态 `state`、进程号 `pid`。
fn status(state: &str, pid: u64) -> serde_json::Value {
    json!({"extensions": [{"package": "other", "state": "running", "pid": 9}, {"package": "miyu-im", "state": state, "pid": pid}]})
}

#[test]
fn the_port_is_trusted_only_from_the_running_process() {
    let board = json!({"pid": 42, "web": 8302});
    assert_eq!(page_port(&status("running", 42), Some(&board)), Some(8302));
    assert_eq!(
        page_port(&status("running", 7), Some(&board)),
        None,
        "别的进程留下的"
    );
    assert_eq!(
        page_port(&status("stopped", 42), Some(&board)),
        None,
        "没在跑"
    );
    assert_eq!(
        page_port(&status("running", 42), None),
        None,
        "没有状态文件"
    );
    assert_eq!(
        page_port(&json!({"extensions": []}), Some(&board)),
        None,
        "没装"
    );
    let no_port = json!({"pid": 42});
    assert_eq!(
        page_port(&status("running", 42), Some(&no_port)),
        None,
        "页面服务没听上"
    );
    let too_big = json!({"pid": 42, "web": 70000});
    assert_eq!(page_port(&status("running", 42), Some(&too_big)), None);
}
