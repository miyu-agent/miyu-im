//! 终端管理员的私聊（施工 O-8，`onebot.md` 第一条「怎么走」第 5 到 10 条）：送进场所会话、记成终端管理员本人；她的回话发回 QQ；同一条
//! 消息再来只算一次，同一个消息编号、时刻不同的是新的一条，没带时刻的照样送（O-8 补）；不是终端管理员的不送也不回；段的数组、
//! CQ 字符串都认，只有图片的不送；同一个号再连进来，新的顶掉旧的。不在白名单里、`venue.binding` 认不出
//! 是终端管理员的是陌生人，不接、不问 `venue.session`；问不到的照核心的（施工 I-65，「施工时定的」第 49 条；测试那一头照核心
//! 拉起的桥的样子答 `venue.binding`）。

use serde_json::{Value, json};

use miyu_im::serve::Notice;
use miyu_session::testkit::{Play, Script};

use crate::support::*;

/// 终端管理员在对应表里的身份。
const ADMIN_ID: &str = "qq:10001";

/// 起一个桥：`venue.binding` 照核心拉起的桥的样子答，`bound` 里的认得（空的是谁都认不出），`None` 的照本机套接字上的核心
/// （不是核心拉起的桥，回 `needs_approval`，问不到）。交回桥和转接。
async fn bound(home: &Home, bound: Option<&'static [&'static str]>) -> (Bridge, Relay) {
    let bindings = bound.map(|bound| Bindings { bound });
    let (serve, relay) = serve_relayed(home.root.clone(), settings(), bindings);
    (start(serve).await, relay)
}

#[tokio::test]
async fn the_admins_private_chat_goes_in_and_her_reply_comes_back() {
    let script = Script::new([Play::Says("在")]);
    let home = Home::new(&script);
    let bridge = bridge(&home).await;
    let mut napcat = admin_napcat(bridge.port).await;
    napcat.admin_says(501, "在吗").await;
    assert_eq!(napcat.reply().await, "在");
    let said = home.said();
    assert_eq!(said.len(), 1, "{said:?}");
    assert_eq!(
        said[0]["by"],
        json!({"kind": "person", "account": "admin", "via": "qq:10001"})
    );
    assert_eq!(said[0]["body"]["blocks"][0]["text"], "在吗");
    assert_eq!(
        said[0]["cause"],
        format!("qq:{BOT}:501:{TIME}"),
        "命令编号拼机器人的号、消息编号和平台给的时刻"
    );
    let created = &home.events(&home.sessions()[0])[0];
    assert_eq!(created["body"]["venue"], "qq:private:10001", "{created}");
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn the_same_message_twice_is_said_once() {
    let script = Script::new([Play::Says("在"), Play::Says("好")]);
    let home = Home::new(&script);
    let bridge = bridge(&home).await;
    let mut napcat = admin_napcat(bridge.port).await;
    napcat.admin_says(7, "在吗").await;
    napcat.admin_says(7, "在吗").await;
    assert_eq!(napcat.reply().await, "在");
    napcat.admin_says(8, "那好").await;
    assert_eq!(napcat.reply().await, "好");
    assert_eq!(home.said_texts(), ["在吗", "那好"]);
    assert_eq!(home.sessions().len(), 1);
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn the_same_message_id_at_another_time_is_another_message() {
    // NapCat 重置本地库以后消息编号从头数，同一个编号又来了：时刻不同，是新的一条，不当成重发吞掉（`chat.md` 第七条第 1 条）。
    let script = Script::new([Play::Says("在"), Play::Says("好")]);
    let home = Home::new(&script);
    let bridge = bridge(&home).await;
    let mut napcat = admin_napcat(bridge.port).await;
    napcat.admin_says(7, "在吗").await;
    assert_eq!(napcat.reply().await, "在");
    let mut again = private_frame(ADMIN, 7, json!("又是我"));
    again["time"] = json!(TIME + 86_400);
    napcat.send(again).await;
    assert_eq!(napcat.reply().await, "好");
    assert_eq!(home.said_texts(), ["在吗", "又是我"]);
    let causes: Vec<_> = home
        .said()
        .iter()
        .map(|said| said["cause"].clone())
        .collect();
    assert_eq!(
        causes,
        [
            json!(format!("qq:{BOT}:7:{TIME}")),
            json!(format!("qq:{BOT}:7:{}", TIME + 86_400)),
        ]
    );
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn a_message_without_time_still_goes_in_with_time_zero() {
    // OneBot v11 的事件都带 `time`；没带的照 0 拼，照样送进去，不吞（`onebot.md`「施工时定的」第 20 条）。
    let script = Script::new([Play::Says("在")]);
    let home = Home::new(&script);
    let bridge = bridge(&home).await;
    let mut napcat = admin_napcat(bridge.port).await;
    let mut frame = private_frame(ADMIN, 12, json!("没带时刻"));
    frame.as_object_mut().expect("是对象").remove("time");
    napcat.send(frame).await;
    assert_eq!(napcat.reply().await, "在");
    let said = home.said();
    assert_eq!(said.len(), 1, "{said:?}");
    assert_eq!(said[0]["cause"], format!("qq:{BOT}:12:0"));
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn a_strangers_private_chat_goes_nowhere() {
    let script = Script::new([Play::Says("在"), Play::Says("嗯")]);
    let home = Home::new(&script);
    let bridge = bridge(&home).await;
    let mut napcat = admin_napcat(bridge.port).await;
    napcat
        .private(
            STRANGER,
            1,
            json!([{"type": "text", "data": {"text": "你好"}}]),
        )
        .await;
    napcat.private(STRANGER, 2, json!("还在吗")).await;
    // 一条条照先后办：终端管理员这一句的回话先到，前面陌生人的两句就是办完了、没回。
    napcat.admin_says(3, "在吗").await;
    assert_eq!(napcat.reply().await, "在");
    // 终端管理员的会话已经有了，陌生人再来也不进它。
    napcat.private(STRANGER, 4, json!("我也在")).await;
    napcat.admin_says(5, "好").await;
    assert_eq!(napcat.reply().await, "嗯");
    assert_eq!(home.said_texts(), ["在吗", "好"]);
    assert_eq!(home.sessions().len(), 1, "陌生人没有会话");
    bridge.stop().await.expect("停得下");
}

/// 段的数组只取字；CQ 码只读字（第 6 条）：只有图的 CQ 码没有字、也认不出带的东西，不送。段的数组里只有图的私聊施工 O-35 起照送
/// （带了东西，`private_media.rs`）。
#[tokio::test]
async fn segments_and_cq_strings_are_read_and_bare_cq_images_are_not_sent() {
    let script = Script::new([Play::Says("一"), Play::Says("二")]);
    let home = Home::new(&script);
    let bridge = bridge(&home).await;
    let mut napcat = admin_napcat(bridge.port).await;
    napcat
        .private(
            ADMIN,
            2,
            json!([
                {"type": "text", "data": {"text": "你"}},
                {"type": "image", "data": {"file": "a.png"}},
                {"type": "text", "data": {"text": "好"}},
            ]),
        )
        .await;
    assert_eq!(napcat.reply().await, "一");
    napcat
        .private(ADMIN, 3, json!("[CQ:image,file=b.png]  "))
        .await;
    napcat
        .private(
            ADMIN,
            4,
            json!("[CQ:face,id=14]在&#91;吗&#93;&#44;&amp;[CQ:at,qq=1]"),
        )
        .await;
    assert_eq!(napcat.reply().await, "二");
    assert_eq!(home.said_texts(), ["你好", "在[吗],&"]);
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn a_new_connection_for_the_same_bot_takes_over() {
    let script = Script::new([Play::Says("在")]);
    let home = Home::new(&script);
    let bridge = bridge(&home).await;
    let heard = |notice: Notice| {
        let notices = std::sync::Arc::clone(&bridge.notices);
        within("桥说了", async move {
            while !notices.lock().expect("没 panic").contains(&notice) {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
    };
    let old = admin_napcat(bridge.port).await;
    heard(Notice::Connected { bot: Some(BOT) }).await;
    let mut new = admin_napcat(bridge.port).await;
    // 旧的那一条断开了：号已经是新的那一条的，不跟着拿掉。
    drop(old);
    heard(Notice::Disconnected { bot: Some(BOT) }).await;
    new.admin_says(9, "在吗").await;
    assert_eq!(new.reply().await, "在");
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn someone_the_binding_does_not_know_is_a_stranger_and_not_asked_about() {
    let home = Home::new(&Script::new([Play::Says("在")]));
    let (bridge, relay) = bound(&home, Some(&[ADMIN_ID])).await;
    let mut napcat = admin_napcat(bridge.port).await;
    napcat.private(STRANGER, 1, json!("在吗")).await;
    napcat.private(STRANGER, 2, json!("/stop")).await;
    napcat.private(STRANGER, 3, json!("还在吗")).await;
    // 一条条照先后办：终端管理员这一句的回话先到，陌生人的三句就是办完了、没回。
    napcat.admin_says(4, "在吗").await;
    assert_eq!(napcat.reply().await, "在");
    assert_eq!(home.said_texts(), ["在吗"], "陌生人的不进任何会话");
    let peers: Vec<Value> = relay
        .sessions()
        .iter()
        .map(|asked| asked["peer"].clone())
        .collect();
    assert_eq!(
        peers,
        [json!(ADMIN_ID)],
        "陌生人的不问 venue.session，免得核心造会话"
    );
    assert!(
        relay.asked().iter().any(|method| method == "venue.binding"),
        "{:?}",
        relay.asked()
    );
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn an_open_strangers_card_without_an_answer_on_the_binding_leaves_it_to_the_core() {
    // 「其他私聊」卡打开了（施工 O-60），核心答不了 `venue.binding`（本机套接字上的桥不是核心拉起的桥）：问不到的照原来办，交给
    // 核心，核心不接对应表里没有的人。
    let home = Home::new(&Script::new([Play::Says("在")]));
    let cards = home.root.state().join("packages").join("im");
    std::fs::create_dir_all(&cards).expect("建得了目录");
    std::fs::write(
        cards.join("cards.json"),
        r#"{"other_privates": {"on": true}}"#,
    )
    .expect("写得进");
    let (bridge, relay) = bound(&home, None).await;
    let mut napcat = admin_napcat(bridge.port).await;
    napcat.private(STRANGER, 1, json!("在吗")).await;
    napcat.private(STRANGER, 2, json!("在吗")).await;
    napcat.admin_says(3, "在吗").await;
    assert_eq!(napcat.reply().await, "在");
    assert!(
        relay.asked().iter().any(|method| method == "venue.binding"),
        "卡打开了才问"
    );
    assert_eq!(home.said_texts(), ["在吗"], "陌生人的不进任何会话");
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn the_admin_is_taken_whether_the_binding_knows_him_or_cannot_be_asked() {
    for known in [Some(&[ADMIN_ID][..]), None] {
        let home = Home::new(&Script::new([Play::Says("在")]));
        let (bridge, relay) = bound(&home, known).await;
        let mut napcat = admin_napcat(bridge.port).await;
        napcat.admin_says(1, "在吗").await;
        assert_eq!(napcat.reply().await, "在", "{known:?}");
        assert_eq!(home.said_texts(), ["在吗"], "{known:?}");
        // 找到场所会话以后、第一轮以前先告诉核心这个场所开着哪些插件（施工 S-1 上c）：多一个 `venue.plugins`。她的回话先入队再发（施工 O-25 中）：多一个 `events.append`。头几个是起来时登记工具（施工 O-26）、后台页的方法（施工 O-28 上）、列名下的场所会话（施工 O-32：进程里的桥不是核心拉起的桥，核心拒了）、订阅软件包列表（施工 O-43 上：挂接点）、列插件（施工 I-8 上）；找会话以前先问对方是不是终端管理员（施工 I-65）。
        assert_eq!(
            relay.asked(),
            [
                "provide",
                "package.methods",
                "venue.sessions",
                "subscribe",
                "content.list",
                "venue.binding",
                "venue.session",
                "subscribe",
                "venue.plugins",
                "session.send",
                "events.append"
            ],
            "{known:?}"
        );
        bridge.stop().await.expect("停得下");
    }
}

#[tokio::test]
async fn the_private_chat_carries_its_venue_fields_and_a_recall_is_noted() {
    // 施工 O-22：私聊也带上场所的格（不写 `ambient`），`show_ids` 照这个私聊的场所规则；私聊的撤回记 `venue.recalled`。
    let script = Script::new([Play::Says("在")]);
    let home = Home::new(&script);
    let dir = home.root.path().join("venues.d");
    std::fs::create_dir_all(&dir).expect("建得了目录");
    let rule =
        format!("[[rule]]\nmatch = {{ kind = \"private\", user = [{ADMIN}] }}\nshow_ids = true\n");
    std::fs::write(dir.join("80-test.toml"), rule).expect("写得进");
    let bridge = bridge(&home).await;
    let mut napcat = admin_napcat(bridge.port).await;
    let message = json!([
        {"type": "reply", "data": {"id": 77}},
        {"type": "text", "data": {"text": "看这个"}},
        {"type": "image", "data": {"file": "p.jpg", "sub_type": 0}},
    ]);
    napcat.private(ADMIN, 501, message).await;
    assert_eq!(napcat.reply().await, "在");
    let said = home.said();
    assert_eq!(said.len(), 1, "{said:?}");
    assert_eq!(said[0]["body"]["blocks"][0]["text"], "看这个");
    assert_eq!(
        said[0]["body"]["venue"],
        json!({
            "msg": "501", "name": "someone", "my_name": "米尤", "reply_to": "77", "media": [{"kind": "image", "id": "p.jpg"}],
            "show_ids": true,
        })
    );
    napcat
        .send(crate::support::group::friend_recall(ADMIN, 501))
        .await;
    let session = home.sessions()[0].clone();
    let recalled = within("记下撤回", async {
        loop {
            if let Some(event) = home
                .events(&session)
                .into_iter()
                .find(|event| event["kind"] == "venue.recalled")
            {
                return event;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    assert_eq!(recalled["body"], json!({"msg": "501", "by": "qq:10001"}));
    bridge.stop().await.expect("停得下");
}
