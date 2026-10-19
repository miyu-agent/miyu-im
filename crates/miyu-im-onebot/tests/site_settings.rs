//! im 页面的设置页（施工 I-7 下一）：进程里跑的桥连真核心，页面经通道读、改通用的两项和账号绑定（核心的 `external.bindings`），在这份登录的
//! 人的连接上，写进系统那一层；别的键不让改。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::browser::{call, logged_in, page_port};
use crate::support::*;

/// `settings.set` 的参数。
fn set(key: &str, value: Value) -> Value {
    json!({"key": key, "value": value})
}

#[tokio::test(flavor = "multi_thread")]
async fn the_settings_page_reads_and_writes_the_general_items_and_bindings() {
    let home = Home::new(&Script::new([]));
    let bridge = start(serve(home.root.clone(), settings())).await;
    let port = page_port(&bridge, 1).await;
    let mut open = logged_in(&home.root, port).await;

    let got = call(&mut open, 1, "settings.get", json!({})).await["result"].clone();
    assert_eq!(got["strip_period"], true, "出厂开：{got}");
    assert_eq!(
        got["recall_receipts"],
        json!(["clear", "clear-memory"]),
        "{got}"
    );
    // 测试的数据根里本来就绑了号：键照页面的写法，不带引号。
    assert_eq!(got["bindings"]["qq:10001"], "admin", "{got}");
    assert_eq!(got["account"], "admin", "{got}");

    let saved = call(
        &mut open,
        2,
        "settings.set",
        set("strip_period", json!(false)),
    )
    .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let added = set("binding:qq:30009", json!("admin"));
    let saved = call(&mut open, 3, "settings.set", added).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let got = call(&mut open, 4, "settings.get", json!({})).await["result"].clone();
    assert_eq!(got["strip_period"], false, "{got}");
    assert_eq!(got["bindings"]["qq:30009"], "admin", "{got}");

    // 去掉一条绑定，别的照旧；别的键不让改。
    let removed = set("binding:qq:30009", Value::Null);
    let saved = call(&mut open, 5, "settings.set", removed).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let got = call(&mut open, 6, "settings.get", json!({})).await["result"].clone();
    assert!(got["bindings"].get("qq:30009").is_none(), "{got}");
    assert_eq!(got["bindings"]["qq:10001"], "admin", "{got}");
    let wrongs = [
        set("whitelist", json!([])),
        set("binding:", json!("admin")),
        set("binding:a\"b", json!("admin")),
        json!({"value": true}),
    ];
    for wrong in wrongs {
        let refused = call(&mut open, 7, "settings.set", wrong.clone()).await;
        assert_eq!(
            refused["error"]["reason"], "bad_params",
            "{wrong} {refused}"
        );
    }
    bridge.stop().await.expect("停得下");
}
