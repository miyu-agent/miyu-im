//! 她在这个场所叫什么（施工 I-38，核心 E-1修三）：群里每条场所消息带 `my_name`（她在这个群的群名片），群名片改了（NapCat 的
//! `group_card` 通知）下一条就带新的，清掉了的下一条重新问 QQ；别人的名片改了不碰她的。私聊带昵称的在 `private.rs`。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::group::*;
use crate::support::*;

/// 这个测试的群。
const GROUP: i64 = 888;

/// 群里说话的人。
const LIN: i64 = 20002;

/// 假 NapCat 认得的群成员：她自己（群名片「米尤」）。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// NapCat 推来的群名片变了：号是 `user` 的人改成 `card`（空的是清掉）。
fn card(user: i64, card: &str) -> Value {
    json!({
        "post_type": "notice", "notice_type": "group_card", "time": TIME, "self_id": BOT,
        "group_id": GROUP, "user_id": user, "card_new": card, "card_old": "米尤",
    })
}

/// 小林说第 `msg` 条，等它记进场所会话，交回那一条的 `my_name`。
async fn spoken(home: &Home, napcat: &Answering, msg: i64) -> Value {
    napcat.send(group_frame(
        GROUP,
        LIN,
        msg,
        json!([plain("在吗")]),
        ("小林", "lin"),
    ));
    let id = msg.to_string();
    let venue = format!("qq:30003:group:{GROUP}");
    let events = until_event(&home.root, &venue, |event| {
        event["kind"] == "message.user" && event["body"]["venue"]["msg"] == id.as_str()
    })
    .await;
    said(&events)
        .iter()
        .find(|event| event["body"]["venue"]["msg"] == id.as_str())
        .map(|event| event["body"]["venue"]["my_name"].clone())
        .expect("记下了")
}

#[tokio::test]
async fn her_name_in_the_group_follows_her_group_card() {
    let script = Script::new([]);
    let rules = format!(
        "[[rule]]\nmatch = {{ kind = \"group\", group = [{GROUP}] }}\nchatty = {{ probability = 0 }}\n"
    );
    let (home, napcat, _) = started(&script, &rules, "", MEMBERS).await;
    assert_eq!(
        spoken(&home, &napcat, 1).await,
        "米尤",
        "问 QQ 问到的群名片"
    );
    napcat.send(card(BOT, "美羽"));
    assert_eq!(
        spoken(&home, &napcat, 2).await,
        "美羽",
        "改了的下一条就带新的"
    );
    napcat.send(card(LIN, "林林"));
    assert_eq!(
        spoken(&home, &napcat, 3).await,
        "美羽",
        "别人的名片不碰她的"
    );
    napcat.send(card(BOT, " "));
    assert_eq!(spoken(&home, &napcat, 4).await, "米尤", "清掉了的重新问");
    assert_eq!(napcat.asked(), [BOT, BOT], "改了的不问，清掉的才再问一次");
    stopped(home).await;
}
