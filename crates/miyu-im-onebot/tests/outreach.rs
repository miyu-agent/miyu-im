//! 主动发消息（施工 O-56，`onebot.md` 第一条「主动发消息」）：真核心拉起真桥、假 NapCat。她调 `chat_send` 发到别的群、白名单成员、
//! 陌生人：照那个场所的出站链（去末尾句号）发、答 `sent`；对方的场所会话找得到的记一块 `reached`，陌生人的不记。别的包经
//! `package.call` 调 `chat.send` 一样发，回 `{sent: <几段>}`，写错的 `bad_params`。

use serde_json::{Value, json};

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::platform::*;
use crate::support::speaking::Lines;
use crate::support::*;

/// 发到的别的群。
const ELSEWHERE: i64 = 999;

/// 陌生人。
const STRANGER: i64 = 50099;

/// 下一个发消息的动作（她在这个群里说的跳过）：交回整个动作。
async fn next_send(napcat: &mut Answering, wanted: &str, key: &str, to: i64) -> Value {
    loop {
        let action = napcat.action().await;
        if action["action"] == wanted && action["params"][key] == to {
            return action;
        }
    }
}

/// 场所 `venue` 的会话里桥记的 `reached` 事实的字。
fn reached(root: &miyu_store::root::DataRoot, venue: &str) -> Vec<String> {
    of_kind(&venue_events(root, venue), "context.injected")
        .iter()
        .filter(|event| event["body"]["kind"] == "reached")
        .filter_map(|event| {
            event["body"]["text"]
                .as_str()
                .map(|text| text.trim_end().to_string())
        })
        .collect()
}

#[tokio::test]
async fn she_messages_another_group_a_friend_and_a_stranger() {
    let lines = Lines::new(
        [
            uses("qq_send", r#"{"group":999,"text":"大家晚上好。"}"#),
            uses("qq_send", r#"{"user":40005,"text":"在吗"}"#),
            uses("qq_send", r#"{"user":50099,"text":"你好"}"#),
            uses("qq_send", r#"{"user":1,"group":2,"text":"x"}"#),
        ]
        .into_iter()
        .flatten(),
    );
    let (home, mut napcat) = up(&lines).await;
    // 终端管理员叫她给别的群发：去掉末尾的句号发过去（事实里写她那句话的开头）、答 `sent`，那个群的会话记一块 `reached`。
    says(
        &home,
        &napcat,
        (ADMIN, "member"),
        101,
        json!([at(BOT), plain(" 去 999 打个招呼")]),
    )
    .await;
    let sent = next_send(&mut napcat, "send_group_msg", "group_id", ELSEWHERE).await;
    assert_eq!(
        sent["params"]["message"],
        json!([{"type": "text", "data": {"text": "大家晚上好"}}])
    );
    assert_eq!(ended(&home, 1).await, done("sent", &[]));
    let fact = std::fs::read_to_string(resources().join("packages/im/data/facts/reached.txt"))
        .expect("读得出")
        .replace("{text}", "大家晚上好")
        .trim_end()
        .to_string();
    assert_eq!(
        reached(&home.root, &format!("qq:group:{ELSEWHERE}")),
        [fact]
    );
    // 白名单成员（她的私聊会话找得到）：发私聊、记 `reached`。
    says(
        &home,
        &napcat,
        (ADMIN, "member"),
        102,
        json!([at(BOT), plain(" 找下小杰")]),
    )
    .await;
    next_send(&mut napcat, "send_private_msg", "user_id", FRIEND).await;
    assert_eq!(ended(&home, 2).await, done("sent", &[]));
    assert_eq!(
        reached(&home.root, &format!("qq:private:{FRIEND}")).len(),
        1
    );
    // 陌生人：照发，没有会话、不记。
    says(
        &home,
        &napcat,
        (ADMIN, "member"),
        103,
        json!([at(BOT), plain(" 找下他")]),
    )
    .await;
    next_send(&mut napcat, "send_private_msg", "user_id", STRANGER).await;
    assert_eq!(ended(&home, 3).await, done("sent", &[]));
    assert!(reached(&home.root, &format!("qq:private:{STRANGER}")).is_empty());
    // 写错的：两个都写了。
    says(
        &home,
        &napcat,
        (ADMIN, "member"),
        104,
        json!([at(BOT), plain(" 乱发")]),
    )
    .await;
    assert_eq!(ended(&home, 4).await, refused("no-target", &[]));
    stopped(home).await;
}

#[tokio::test]
async fn other_packages_send_in_her_name() {
    let (home, mut napcat) = up(&Lines::new([])).await;
    let mut head = Head::connect(&home.root).await;
    let call = |params: Value| json!({"package": "im", "method": "chat.send", "params": params});
    let reply = head
        .call(
            "package.call",
            call(json!({"group": ELSEWHERE, "text": "公告：今晚八点开会。"})),
        )
        .await;
    assert_eq!(reply["result"], json!({"sent": 1}), "{reply}");
    let sent = next_send(&mut napcat, "send_group_msg", "group_id", ELSEWHERE).await;
    assert_eq!(
        sent["params"]["message"][0]["data"]["text"],
        "公告：今晚八点开会"
    );
    let wrong = head.call("package.call", call(json!({"text": "x"}))).await;
    assert_eq!(wrong["error"]["data"]["message"], "bad_params", "{wrong}");
    // 别的平台的不发（施工 I-1）；写明是 qq 的照发。
    let other = head
        .call(
            "package.call",
            call(json!({"group": ELSEWHERE, "text": "x", "platform": "telegram"})),
        )
        .await;
    assert_eq!(other["error"]["data"]["message"], "bad_params", "{other}");
    let ours = head
        .call(
            "package.call",
            call(json!({"group": ELSEWHERE, "text": "第二条", "platform": "qq"})),
        )
        .await;
    assert_eq!(ours["result"], json!({"sent": 1}), "{ours}");
    let sent = next_send(&mut napcat, "send_group_msg", "group_id", ELSEWHERE).await;
    assert_eq!(sent["params"]["message"][0]["data"]["text"], "第二条");
    stopped(home).await;
}
