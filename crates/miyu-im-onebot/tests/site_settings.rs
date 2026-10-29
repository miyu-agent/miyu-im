//! im 页面的设置页（施工 I-7 下一）：进程里跑的桥连真核心，页面经通道读、改通用的两项和本人的平台 ID（核心的 `external.bindings`，
//! 施工 I-68：只交身份，加的写 `owner`），在这份登录的人的连接上，写进系统那一层；别的键不让改。

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
    assert!(
        got.get("strip_period").is_none() && got.get("humanlike").is_none(),
        "去掉末尾句号、分条发送挪到会话卡片了（施工 I-49）：{got}"
    );
    assert_eq!(
        got["recall_receipts"],
        json!(["clear", "clear-memory"]),
        "{got}"
    );
    // 测试的数据根里本来就绑了号：照页面的写法，不带引号；写 `owner` 的、老的 `admin` 的都算，只交身份。
    assert_eq!(got["bindings"], json!(["qq:10001", "qq:10007"]), "{got}");
    assert!(
        got.get("account").is_none(),
        "登录不带账号了（施工 I-65）：{got}"
    );

    let saved = call(
        &mut open,
        2,
        "settings.set",
        set("recall_receipts", json!(["clear"])),
    )
    .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let added = set("binding:qq:30009", json!(true));
    let saved = call(&mut open, 3, "settings.set", added).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let got = call(&mut open, 4, "settings.get", json!({})).await["result"].clone();
    assert_eq!(got["recall_receipts"], json!(["clear"]), "{got}");
    assert!(
        got["bindings"]
            .as_array()
            .is_some_and(|ids| ids.contains(&json!("qq:30009"))),
        "{got}"
    );
    let written = std::fs::read_to_string(home.root.path().join("config.toml")).expect("读得出");
    assert!(
        written.contains("\"qq:30009\" = \"owner\""),
        "加的写 owner：{written}"
    );

    // 去掉一条绑定，别的照旧；别的键不让改。
    let removed = set("binding:qq:30009", Value::Null);
    let saved = call(&mut open, 5, "settings.set", removed).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let got = call(&mut open, 6, "settings.get", json!({})).await["result"].clone();
    assert_eq!(got["bindings"], json!(["qq:10001", "qq:10007"]), "{got}");
    let wrongs = [
        set("whitelist", json!([])),
        set("humanlike", json!(false)),
        set("binding:", json!(true)),
        set("binding:a\"b", json!(true)),
        // 施工 I-85 补：身份得是 `<平台>:<号>`。
        set("binding:qq", json!(true)),
        set("binding:qq:1 2", json!(true)),
        set("binding:Q Q:1", json!(true)),
        set("binding:qq:30009", json!("admin")),
        set("binding:qq:30009", json!(false)),
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
