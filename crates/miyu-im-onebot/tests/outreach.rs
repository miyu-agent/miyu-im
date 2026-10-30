//! 主动发消息（施工 O-56，`onebot.md` 第一条「主动发消息」）：真核心拉起真桥、假 NapCat。她调 `chat_send` 发到别的群、白名单成员、
//! 陌生人：照那个场所的出站链（去末尾句号）发、答 `sent`；对方的场所会话找得到的记一块 `reached`，陌生人的不记。别的包经
//! `package.call` 调 `chat.send` 一样发，回 `{sent: <几段>}`，写错的 `bad_params`。本机的会话里她也能调（施工 I-40）。

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
    let fact = std::fs::read_to_string(resources().join("packages/miyu-im/data/facts/reached.txt"))
        .expect("读得出")
        .replace("{text}", "大家晚上好")
        .trim_end()
        .to_string();
    assert_eq!(
        reached(&home.root, &format!("qq:30003:group:{ELSEWHERE}")),
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
        reached(&home.root, &format!("qq:30003:private:{FRIEND}")).len(),
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
    assert!(reached(&home.root, &format!("qq:30003:private:{STRANGER}")).is_empty());
    // 写错的：两个都写了。
    says(
        &home,
        &napcat,
        (ADMIN, "member"),
        104,
        json!([at(BOT), plain(" 乱发")]),
    )
    .await;
    assert_eq!(ended(&home, 4).await, refused("no-recipient", &[]));
    stopped(home).await;
}

#[tokio::test]
async fn other_packages_send_in_her_name() {
    let (home, mut napcat) = up(&Lines::new([])).await;
    let mut head = Head::connect(&home.root).await;
    let call =
        |params: Value| json!({"package": "miyu-im", "method": "chat.send", "params": params});
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

/// 造一个本机的会话（终端、网页那种），切成完全放开，叫她做 `text`。
async fn local_session(home: &Home, text: &str) {
    let mut core = miyu_client::open::Core::connect_running(&home.root, "test")
        .await
        .expect("连得上核心");
    let cwd = home.root.path().to_string_lossy().into_owned();
    let made = core
        .call("local-1", "session.create", json!({"cwd": cwd}))
        .await
        .expect("造得出");
    let session = made["session"].clone();
    let full = json!({"session": session, "level": "full"});
    core.call("local-2", "session.set_permission_level", full)
        .await
        .expect("切得了");
    let asked = json!({"session": session, "text": text});
    core.call("local-3", "session.send", asked)
        .await
        .expect("发得进");
}

/// 等她拿到第二次请求（工具的结果交回去了），交回她发出的请求。
async fn second_request(lines: &Lines) -> Vec<miyu_kernel::request::Request> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
    while lines.requests().len() < 2 {
        assert!(tokio::time::Instant::now() < deadline, "等不到她拿到结果");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    lines.requests()
}

#[tokio::test]
async fn from_a_local_session_she_messages_a_group() {
    // 施工 I-40：终端、网页里叫她去 QQ 上发。本机的会话也有 `qq_send`（完全放开的会话照对外发消息直接放行），经连着的号发，答 `sent`。
    let lines = Lines::new(uses("qq_send", r#"{"group":999,"text":"大家晚上好。"}"#));
    let (home, mut napcat) = up(&lines).await;
    local_session(&home, "去 999 打个招呼").await;
    let sent = next_send(&mut napcat, "send_group_msg", "group_id", ELSEWHERE).await;
    assert_eq!(
        sent["params"]["message"],
        json!([{"type": "text", "data": {"text": "大家晚上好"}}]),
        "照那个群的出站链去掉末尾句号"
    );
    let requests = second_request(&lines).await;
    assert!(
        requests[0].tools.iter().any(|tool| tool.name == "qq_send"),
        "本机的会话的工具面里有它"
    );
    assert!(
        format!("{:?}", requests[1]).contains(sentence("sent", &[]).trim()),
        "答了 sent"
    );
    stopped(home).await;
}

#[tokio::test]
async fn from_a_local_session_with_qq_disconnected_she_is_told_it_is_unreachable() {
    // 施工 I-43（项目主人 10-27）：QQ 没连着时 `qq_send` 照样在工具面里（工具面会话内不变，前缀缓存不掰），调了答够不着。
    let lines = Lines::new(uses("qq_send", r#"{"group":999,"text":"大家晚上好。"}"#));
    let (home, napcat) = up(&lines).await;
    drop(napcat);
    until_logged(&home.root, |line| line.contains("napcat disconnected")).await;
    local_session(&home, "去 999 打个招呼").await;
    let requests = second_request(&lines).await;
    assert!(
        requests[0].tools.iter().any(|tool| tool.name == "qq_send"),
        "没连着也在工具面里"
    );
    assert!(
        format!("{:?}", requests[1]).contains(sentence("unreachable", &[]).trim()),
        "答了够不着：{:?}",
        requests[1]
    );
    stopped(home).await;
}

#[tokio::test]
async fn from_a_local_session_with_two_bots_she_says_which_one_sends() {
    // 施工 I-81 下下（设计 07 第 7.7 节第 5 条）：同一个平台连着两个号，本机的会话里没写 `bot` 的不猜，答有哪几个号；写了的经它发。
    const OTHER: i64 = 30004;
    let lines = Lines::new([
        crate::support::speaking::Line::uses(
            "",
            "qq_send",
            r#"{"group":999,"text":"大家晚上好。"}"#,
        ),
        crate::support::speaking::Line::uses(
            "",
            "qq_send",
            r#"{"group":999,"text":"大家晚上好。","bot":30004}"#,
        ),
        crate::support::speaking::Line::says("好。"),
    ]);
    let (home, _first, listen) = started_ranked(
        std::sync::Arc::new(lines.clone()),
        &rules(),
        &onebot(),
        (MEMBERS, RANKS),
    )
    .await;
    let mut other = crate::support::napcat::napcat(
        listen,
        "/onebot/v11/ws",
        crate::support::napcat::Auth::Bearer(TOKEN),
        Some(OTHER),
    )
    .await
    .expect("接了")
    .answering(MEMBERS);
    until_logged(&home.root, |line| {
        line.contains("napcat connected") && line.contains("30004")
    })
    .await;
    local_session(&home, "去 999 打个招呼").await;
    let sent = next_send(&mut other, "send_group_msg", "group_id", ELSEWHERE).await;
    assert_eq!(
        sent["params"]["message"],
        json!([{"type": "text", "data": {"text": "大家晚上好"}}]),
        "写了 bot 的经它发"
    );
    let requests = lines.requests();
    let which = sentence("which-bot", &[("bots", &format!("{BOT}, {OTHER}"))]);
    assert!(
        format!("{:?}", requests[1]).contains(which.trim()),
        "没写 bot 的答有哪几个号：{:?}",
        requests[1]
    );
    stopped(home).await;
}
