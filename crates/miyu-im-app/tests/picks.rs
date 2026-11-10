//! `/model`、`/persona`、`/preset` 照这个会话的卡片读写（施工 I-194，项目主人 11-10）：卡片挂在这个连接的号下面的（后台会话页加的
//! 都是）也换得了；人格、预设和模型一个样，带编号列、照编号换。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::*;

/// 读卡片文件里私聊 10001 的那张卡（只该有一张）。
fn own_card(dir: &std::path::Path) -> Value {
    let text = std::fs::read_to_string(dir.join("cards.json")).expect("读得到");
    let cards: Value = serde_json::from_str(&text).expect("是 JSON");
    let mine: Vec<&Value> = cards["privates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|card| card["id"] == 10001)
        .collect();
    assert_eq!(mine.len(), 1, "只有一张：{cards}");
    mine[0].clone()
}

#[tokio::test]
async fn slash_model_switches_a_card_that_belongs_to_this_bot() {
    // 项目主人 11-10：私聊的卡片是后台会话页加的，挂在机器人号下面；`/model 2` 回「这个会话没有自己的卡片」。原来只照整个平台的
    // 那张找。
    let home = Home::new(&Script::new([]));
    let dir = miyu_im::dirs::state(&home.root);
    std::fs::create_dir_all(&dir).expect("建得了");
    let cards = json!({"privates": [{"id": 10001, "bot": BOT, "set": {"model": "@flagship"}}]});
    std::fs::write(dir.join("cards.json"), cards.to_string()).expect("写得进");
    let bridge = bridge(&home).await;
    let mut napcat = admin_napcat(bridge.port).await;
    napcat.admin_says(1, "/model").await;
    assert!(
        napcat.reply().await.starts_with("现在用的是 @flagship\n"),
        "读的也是这个号的那张"
    );
    napcat.admin_says(2, "/model 2").await;
    assert_eq!(napcat.reply().await, "换成 @lite 了");
    let card = own_card(&dir);
    assert_eq!(card["set"]["model"], "@lite", "{card}");
    assert_eq!(card["bot"], BOT, "改的是原来那张，没另建");
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn slash_persona_and_preset_list_with_numbers_and_switch_by_number() {
    // 项目主人 11-10：QQ 里没有 /persona、/preset，要和 /model 一个样。
    let home = Home::new(&Script::new([]));
    let dir = miyu_im::dirs::state(&home.root);
    let bridge = bridge(&home).await;
    let mut napcat = admin_napcat(bridge.port).await;
    for (say, command, key) in [(10, "/persona", "persona"), (20, "/preset", "preset")] {
        napcat.admin_says(say, command).await;
        let said = napcat.reply().await;
        assert!(said.starts_with("现在跟随默认"), "{said}");
        assert!(
            said.lines()
                .any(|line| line == format!("{command} <编号>：切换")),
            "{said}"
        );
        napcat
            .admin_says(say + 1, &format!("{command} search"))
            .await;
        let found = napcat.reply().await;
        assert!(
            found.starts_with(&format!("用 {command} <编号> 切换：")),
            "{found}"
        );
        // 第一行是「1. 名字 (编号)」或者「1. 编号」：取出编号。
        let first = found
            .lines()
            .find_map(|line| line.strip_prefix("1. "))
            .unwrap_or_else(|| panic!("带编号：{found}"));
        let shown = first.trim_end_matches("（正在用）");
        let id = match shown.rsplit_once(" (") {
            Some((_, id)) => id.trim_end_matches(')'),
            None => shown,
        };
        napcat.admin_says(say + 2, &format!("{command} 1")).await;
        assert_eq!(napcat.reply().await, format!("换成 {shown} 了"));
        let card = own_card(&dir);
        assert_eq!(card["set"][key], id, "写进了这个会话的卡片：{card}");
        napcat.admin_says(say + 3, command).await;
        assert!(
            napcat
                .reply()
                .await
                .starts_with(&format!("现在用的是 {shown}\n"))
        );
        napcat.admin_says(say + 4, &format!("{command} 默认")).await;
        assert_eq!(napcat.reply().await, "改回跟随默认了");
        assert_eq!(own_card(&dir)["set"][key], Value::Null);
    }
    bridge.stop().await.expect("停得下");
}
