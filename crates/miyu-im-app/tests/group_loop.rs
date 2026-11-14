//! 群里的 `/loop` 是自主循环的开关（施工 I-239，项目主人 11-14 定）：桥自己办、不交核心。光打 `/loop` 打开这个群卡片上的
//! `glance`，`/loop off` 关上，带间隔、带话的回一句用法；回执不撤回。只有终端管理员和 `managers` 里的人能用，别人打的什么都不回。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::group::*;
use crate::support::*;

/// 群号。
const GROUP: i64 = 888;

/// 群里的别人：用不了。
const LIN: i64 = 20002;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 群里 `who` 说 `text`，编号 `id`。
fn group_says(who: i64, id: i64, text: &str) -> Value {
    let name = if who == ADMIN {
        ("终端管理员", "o")
    } else {
        ("小林", "lin")
    };
    group_frame(GROUP, who, id, json!([plain(text)]), name)
}

/// 一个发群消息的动作里的字。
fn text_of(action: &Value) -> &str {
    action["params"]["message"][0]["data"]["text"]
        .as_str()
        .expect("有字")
}

/// 卡片文件里这个群的 `glance`。
fn glance(home: &Home) -> Value {
    let path = miyu_im::dirs::state(&home.root).join("cards.json");
    let cards: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("读得到")).expect("是 JSON");
    let mine: Vec<&Value> = cards["groups"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|card| card["id"] == GROUP)
        .collect();
    assert_eq!(mine.len(), 1, "只有一张：{cards}");
    mine[0]["set"]["glance"].clone()
}

#[tokio::test]
async fn slash_loop_in_a_group_switches_the_autonomous_loop_and_nothing_else() {
    let (home, mut napcat, _) = started(&Script::new([]), "", "", MEMBERS).await;
    napcat.send(group_says(ADMIN, 60, "/loop"));
    let on = napcat.action().await;
    assert_eq!(
        (on["action"].as_str(), text_of(&on)),
        (Some("send_group_msg"), "这个群的自主循环开了"),
        "{on}"
    );
    assert_eq!(glance(&home), true);
    napcat.send(group_says(ADMIN, 61, "/loop 5m 看看群里"));
    assert_eq!(
        text_of(&napcat.action().await),
        "群里只有 /loop（开自主循环）和 /loop off（关）"
    );
    assert_eq!(glance(&home), true, "用法不改卡片");
    // 别人打的：什么都不回，卡片不动。接着管理员关上，下一个动作就是关上的回执。
    napcat.send(group_says(LIN, 62, "/loop off"));
    napcat.send(group_says(ADMIN, 63, "/loop off"));
    assert_eq!(
        text_of(&napcat.action().await),
        "这个群的自主循环关了",
        "小林的没回"
    );
    assert_eq!(glance(&home), false);
    // 回执不撤，也没交给核心（核心的 `/loop` 在群里不跑）。
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    assert!(napcat.pending_recall().is_none(), "回执不撤");
    assert!(napcat.pending().is_none(), "小林的什么都没发");
    let ran = home
        .sessions()
        .iter()
        .flat_map(|session| home.events(session))
        .filter(|event| event["kind"] == "command.ran")
        .count();
    assert_eq!(ran, 0, "没交给核心");
    stopped(home).await;
}
