//! 后台页调的方法（施工 O-28 上，`onebot.md` 第一条「后台页」第 2 条）：登记的参数；桥手里的状态还没交进来的、不认识的方法怎么回。
//! 交进状态以后答的 `status`、`connection.token` 要一个跑着的桥，在集成测试里（`tests/backstage.rs`、`apply.rs`、`no_token.rs`）。

use serde_json::json;

use super::{Methods, exported, registered};

#[test]
fn four_methods_are_registered_without_a_timeout() {
    // 后台页的两个、给别的包调的两个（施工 O-42）。
    assert_eq!(
        registered(),
        json!({"methods": [{"name": "status"}, {"name": "connection.token"}, {"name": "qq.action"}, {"name": "qq.venue"}]})
    );
}

#[test]
fn only_the_two_for_other_packages_go_to_the_route() {
    let routed: Vec<_> = [
        "status",
        "connection.token",
        "qq.action",
        "qq.venue",
        "qq.inbound",
        "",
    ]
    .into_iter()
    .filter(|method| exported(method))
    .collect();
    assert_eq!(routed, ["qq.action", "qq.venue"]);
}

#[test]
fn unknown_methods_and_methods_before_ready_are_unknown() {
    let unknown =
        json!({"code": -32601, "message": "unknown_method", "data": {"reason": "unknown_method"}});
    let methods = Methods::new();
    for (id, method) in [
        (json!("core-1"), "status"),
        (json!(7), "connection.token"),
        (json!("core-2"), "apply"),
    ] {
        assert_eq!(
            methods.answer(id.clone(), &json!({"method": method, "params": {}})),
            json!({"jsonrpc": "2.0", "id": id, "error": unknown}),
            "{method}"
        );
    }
    assert_eq!(
        methods.answer(json!(8), &json!({})),
        json!({"jsonrpc": "2.0", "id": 8, "error": unknown}),
        "没写方法名"
    );
}
