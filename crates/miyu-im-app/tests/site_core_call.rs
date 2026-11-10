//! im 页面的转发口 `core.call`（设计 09，施工 I-145、I-148）：进程里跑的桥连真核心，页面经通道调核心的方法。白名单里的原样转、交回
//! `result`；白名单外的拒 `not_allowed`；核心拒了的交回 `refused`，带核心拒绝里的整份 `data`（核心 CA-2 以后才有）。

use serde_json::json;

use miyu_session::testkit::Script;

use crate::support::browser::{call, logged_in, page_port};
use crate::support::*;

#[tokio::test(flavor = "multi_thread")]
async fn the_page_calls_listed_core_methods_and_gets_refusals_with_their_data() {
    let home = Home::new(&Script::new([]));
    let bridge = start(serve(home.root.clone(), settings())).await;
    let port = page_port(&bridge, 1).await;
    let mut open = logged_in(&home.root, port).await;

    let listed = call(&mut open, 1, "core.call", json!({"method": "persona.list"})).await;
    assert!(
        listed["result"]["result"]["personas"].is_array(),
        "{listed}"
    );

    let refused = call(
        &mut open,
        2,
        "core.call",
        json!({"method": "session.send", "params": {}}),
    )
    .await;
    assert_eq!(refused["error"]["reason"], "not_allowed", "{refused}");

    // 看人格的记忆（施工 I-207）：读的放行（核心接了、没装记忆的照核心的拒绝交回，都不是 im 自己拒的）；改的、删的不放。
    let cards = call(
        &mut open,
        4,
        "core.call",
        json!({"method": "memory.cards", "params": {}}),
    )
    .await;
    assert!(cards["error"].is_null(), "读认识的人的卡放行：{cards}");
    for (id, method) in [
        (5, "memory.forget"),
        (6, "memory.card_update"),
        (7, "memory.update"),
    ] {
        let refused = call(
            &mut open,
            id,
            "core.call",
            json!({"method": method, "params": {}}),
        )
        .await;
        assert_eq!(
            refused["error"]["reason"], "not_allowed",
            "{method}：{refused}"
        );
    }

    // 写了对不上的 expect：核心回 config_conflict，data.current 是这一项现在的样子（没写过的是 {}）。
    let change = json!({"key": "ui.language", "value": "en", "expect": {"value": "zz"}});
    let params = json!({"layer": "system", "changes": [change]});
    let conflict = call(
        &mut open,
        3,
        "core.call",
        json!({"method": "config.set", "params": params}),
    )
    .await;
    let refused = &conflict["result"]["refused"];
    assert_eq!(refused["reason"], "config_conflict", "{conflict}");
    assert!(
        refused["data"]["current"].is_object(),
        "核心拒绝里的 data 整份带回：{conflict}"
    );
}
