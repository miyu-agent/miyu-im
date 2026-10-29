//! 原来的三个配置项挪进会话默认值（施工 I-7 下二）：配置里写着睡觉时间、群里的触发词、她看得到 QQ 号的，桥起来时搬进 `cards.json` 的
//! `defaults`，配置里的去掉。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::*;

/// 她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

#[tokio::test(flavor = "multi_thread")]
async fn the_old_settings_move_into_the_chat_defaults() {
    let old = "sleep = \"01:00-02:00\"\nkeywords = [\"小米\"]\nshow_ids = true\n";
    let (home, _napcat, _) = started(&Script::new([]), "", old, MEMBERS).await;
    within("搬过去", async {
        while !run_log(&home.root).contains("settings moved into chat defaults") {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    let path = home.root.state().join("packages/miyu-im/cards.json");
    let cards: Value =
        serde_json::from_slice(&std::fs::read(path).expect("读得出")).expect("是 JSON");
    assert_eq!(
        cards["defaults"],
        json!({"sleep": "01:00-02:00", "keywords": ["小米"], "show_ids": true})
    );
    let mut head = Head::connect(&home.root).await;
    let got = head
        .call(
            "config.get",
            json!({"keys": ["im.sleep", "im.keywords", "im.show_ids"]}),
        )
        .await;
    let items = &got["result"]["items"];
    for key in ["im.sleep", "im.keywords", "im.show_ids"] {
        assert_ne!(items[key]["origin"]["layer"], "system", "去掉了：{got}");
    }
    stopped(home).await;
}
