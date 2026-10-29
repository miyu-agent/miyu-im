//! 她发出去的图（施工 O-51，`onebot.md` 第一条「发图」；核心 X-19、设计 34 第二节第 5 条）：真核心拉起真桥、假 NapCat，另装一个
//! 测试包（`miyu-im-test-hook` 带 `sticker:`）登记一件工具 `sticker`，她调了答 `sent` 里一张表情包。群里、私聊里照她调的先后
//! 发：先甩图再说字；图是带 `sub_type=1`、`subType=1` 的图片段，内容是那张图；群里发出去了记 `venue.delivered`（`images`）。

use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use tokio::sync::oneshot;

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::hooked::install_with;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 群号。
const GROUP: i64 = 669;

/// 假 NapCat 认得的群成员：她自己、终端管理员。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (ADMIN, "老板", "boss")];

/// 一张 1×1 的 PNG（同 `shown.rs`）。
const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0,
    0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

/// 那张图写进一个临时文件：测试包照它 `blob.put`。
fn picture() -> PathBuf {
    let (dir, _) = temp_root();
    let path = dir.join("cat.png");
    std::fs::write(&path, PNG).expect("写得进");
    path
}

/// 她这一轮先调 `sticker`、再说一句。
fn script() -> Script {
    Script::new([Play::calls(&[("sticker", "{}")]), Play::Says("看这个")])
}

/// 等到测试包登记了 `sticker`（核心写了它的登记缓存）：不然她的第一轮可能比它早，工具面上还没有。
async fn until_provided(home: &Home) {
    let cache = home
        .root
        .path()
        .join("state")
        .join("providers")
        .join("stickers.json");
    let deadline = tokio::time::Instant::now() + WAIT;
    while !cache.exists() {
        assert!(tokio::time::Instant::now() < deadline, "测试包没登记工具");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

/// 一条发出去的消息是不是那张表情包：一个图片段，带两种拼写的 `sub_type`，内容是那张图。
fn is_the_sticker(message: &[Value]) {
    assert_eq!(message.len(), 1, "{message:?}");
    let data = &message[0]["data"];
    assert_eq!(message[0]["type"], "image", "{message:?}");
    assert_eq!(data["sub_type"], 1);
    assert_eq!(data["subType"], 1);
    assert_eq!(data["file"], format!("base64://{}", STANDARD.encode(PNG)));
}

#[tokio::test]
async fn a_sticker_she_sends_reaches_the_group_before_the_words_after_it() {
    let path = picture();
    let args = vec![format!("sticker:{}", path.display())];
    let install =
        move |root: &miyu_store::root::DataRoot| install_with(root, "stickers", &args, &["tools"]);
    let (home, mut napcat, _) = started_installed(&script(), "", MEMBERS, install).await;
    until_provided(&home).await;
    let words = json!([at(BOT), plain(" 来个表情")]);
    napcat.send(group_frame(GROUP, ADMIN, 1, words, ("老板", "boss")));
    is_the_sticker(&napcat.group_message(GROUP).await);
    // 字跟在她自己的表情包后面：群里最后一条是她的，照规则引用老板那一条（施工 I-64：照交出去那一刻算；原来写好那一刻表情包的
    // 回执还没回来，算成不是她的、不引用）。
    let words = napcat.group_message(GROUP).await;
    let text: String = words
        .iter()
        .filter_map(|segment| segment["data"]["text"].as_str())
        .collect();
    assert_eq!(text, "看这个", "{words:?}");
    let events = until_events(&home.root, &format!("qq:30003:group:{GROUP}"), |events| {
        of_kind(events, "venue.delivered").len() == 2
    })
    .await;
    let delivered = of_kind(&events, "venue.delivered");
    let sent = of_kind(&events, "tool.result")[0]["body"]["sent"][0]["blob"].clone();
    assert_eq!(
        delivered[0]["body"]["images"],
        json!([sent]),
        "图记进 venue.delivered"
    );
    assert_eq!(delivered[0]["body"]["text"], "");
    let queued: Vec<Value> = of_kind(&events, "ext.im.venues.queued")
        .iter()
        .map(|one| one["body"]["kind"].clone())
        .collect();
    assert_eq!(queued, ["image", "reply"], "图也走出站队列，照先后");
    stopped(home).await;
}

/// 白名单成员（她的私聊关在沙盒里，同群会话）。
const FRIEND: i64 = 50005;

#[tokio::test]
async fn a_sticker_reaches_a_whitelisted_private_chat_too() {
    let path = picture();
    let args = vec![format!("sticker:{}", path.display())];
    let install =
        move |root: &miyu_store::root::DataRoot| install_with(root, "stickers", &args, &["tools"]);
    let onebot = format!("whitelist = [\"qq:{FRIEND}\"]\n");
    let (home, mut napcat, _) = started_installed_with(&script(), &onebot, MEMBERS, install).await;
    until_provided(&home).await;
    napcat.send(private_frame(FRIEND, 1, json!([plain("来个表情")])));
    let first = napcat.action().await;
    assert_eq!(first["action"], "send_private_msg", "{first}");
    assert_eq!(first["params"]["user_id"], FRIEND);
    is_the_sticker(first["params"]["message"].as_array().expect("段的数组"));
    let second = napcat.action().await;
    assert_eq!(second["params"]["message"][0]["data"]["text"], "看这个");
    stopped(home).await;
}

#[tokio::test]
async fn a_sticker_that_never_got_out_is_noted_for_her() {
    // 群里被禁言着、排着的过了期限（测试的资源目录把 `queue_expire_seconds` 改成 1）：图不发，记一块 `unshown`（`why=expired`）。
    let path = picture();
    let args = vec![format!("sticker:{}", path.display())];
    let install =
        move |root: &miyu_store::root::DataRoot| install_with(root, "stickers", &args, &["tools"]);
    let tuned = json!({"queue_expire_seconds": 1, "mute_window_seconds": 0});
    // 施工 I-33：她的话要在禁言以后才出来，照真路径先开一轮压着，禁言了再放行（不再以人的身份代判官开轮）。
    let (release, held) = oneshot::channel();
    let lines = Lines::new([
        Line::uses("", "sticker", "{}").released_by(held),
        Line::says("看这个"),
    ]);
    let (home, napcat, _) =
        started_installed_tuned_by(Arc::new(lines), &tuned, MEMBERS, install).await;
    until_provided(&home).await;
    let venue = format!("qq:30003:group:{GROUP}");
    // 终端管理员 @ 她开一轮，她调 `sticker`、再说一句：禁言了才放行，都排着，过期作废。
    napcat.send(group_frame(
        GROUP,
        ADMIN,
        1,
        json!([at(BOT), plain(" 在吗")]),
        ("老板", "boss"),
    ));
    until_events(&home.root, &venue, |events| {
        !of_kind(events, "turn.started").is_empty()
    })
    .await;
    napcat.send(group_ban(GROUP, BOT, "ban", Some(600)));
    until_events(&home.root, &venue, |events| {
        !of_kind(events, "ext.im.venues.muted").is_empty()
    })
    .await;
    release.send(()).expect("她在等");
    let events = until_events(&home.root, &venue, |events| {
        of_kind(events, "context.injected")
            .iter()
            .any(|event| event["body"]["kind"] == "unshown")
    })
    .await;
    let unshown: Vec<String> = of_kind(&events, "context.injected")
        .iter()
        .filter(|event| event["body"]["kind"] == "unshown")
        .filter_map(|event| event["body"]["text"].as_str().map(str::to_string))
        .collect();
    assert_eq!(unshown.len(), 1, "{unshown:?}");
    assert!(unshown[0].contains(r#"why="expired""#), "{}", unshown[0]);
    assert!(
        unshown[0].contains("A cat waving."),
        "名字照 alt：{}",
        unshown[0]
    );
    stopped(home).await;
}

#[tokio::test]
async fn a_sticker_reaches_the_admins_private_chat() {
    // 终端管理员的私聊归他本人：核心 W-6 再补（4a691ddc）起 `blob.get` 认 `sent` 里的 blob（施工 O-51 补）。
    let path = picture();
    let args = vec![format!("sticker:{}", path.display())];
    let install =
        move |root: &miyu_store::root::DataRoot| install_with(root, "stickers", &args, &["tools"]);
    let (home, mut napcat, _) = started_installed(&script(), "", MEMBERS, install).await;
    until_provided(&home).await;
    napcat.send(private_frame(ADMIN, 1, json!([plain("来个表情")])));
    let first = napcat.action().await;
    assert_eq!(first["action"], "send_private_msg", "{first}");
    assert_eq!(first["params"]["user_id"], ADMIN);
    is_the_sticker(first["params"]["message"].as_array().expect("段的数组"));
    let second = napcat.action().await;
    assert_eq!(second["params"]["message"][0]["data"]["text"], "看这个");
    let log = run_log(&home.root);
    assert!(!log.contains("image not sent"), "{log}");
    stopped(home).await;
}
