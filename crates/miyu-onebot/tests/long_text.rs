//! 长文转成图（施工 O-59 上，`onebot.md`「场所卡片」的「长文」、「出站队列」）：真核心拉起真桥、假 NapCat，另装一个测试包扮渲染包
//! （`miyu-render`，`render:` 答两页）。出厂超过 300 个字的回话画成图：一页一条图片段、照先后，渲染包收到的是全文；入队第一页记成
//! `reply`（全文）、第二页记成 `image`；`venue.delivered` 第一页带全文和回的人、第二页都是空的；她 `mention` 的 @ 只在第一页。
//! 判、画、记照过了出站链的全文（去了末尾的「。」）。不长的照发字；渲染包没装的、交回零页的、这个群写了
//! `long_text = "split"` 的照发字。

use std::path::PathBuf;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::hooked::{install, install_with};
use crate::support::*;

/// 群号。
const GROUP: i64 = 671;

/// 假 NapCat 认得的群成员：她自己、终端管理员。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (ADMIN, "老板", "boss")];

/// 一张 1×1 的 PNG（同 `stickers.rs`）。
const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0,
    0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

/// 一段 302 个字的长文，末尾一个「。」：出站链去掉它以后正好 301 个字。
static LONG: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    let head = "## 长回复\n\n";
    format!("{head}{}。", "长".repeat(301 - head.chars().count()))
});

/// 她说的那段长文。
fn long() -> &'static str {
    LONG.as_str()
}

/// 过了出站链的样子：去掉末尾的「。」（判长文、画、记下都照它）。
fn cleaned() -> &'static str {
    LONG.strip_suffix('。').expect("末尾是句号")
}

/// 装扮渲染包的测试包：图写进临时文件，收到的调用记进 `record`。
fn renderer(record: &std::path::Path) -> impl Fn(&miyu_store::root::DataRoot) + Clone {
    let (dir, _) = temp_root();
    let path: PathBuf = dir.join("page.png");
    std::fs::write(&path, PNG).expect("写得进");
    let args = vec![
        format!("record:{}", record.display()),
        format!("render:{}", path.display()),
    ];
    move |root: &miyu_store::root::DataRoot| {
        install_with(root, "miyu-render", &args, &[]);
        export_markdown(root);
    }
}

/// 测试包的清单导出 `markdown`（`install_with` 不导出方法）。
fn export_markdown(root: &miyu_store::root::DataRoot) {
    let manifest = root
        .path()
        .join("home/admin/packages/miyu-render/package.toml");
    let text = std::fs::read_to_string(&manifest).expect("读得出");
    std::fs::write(
        &manifest,
        text.replace("methods = []", "methods = [\"markdown\"]"),
    )
    .expect("写得进");
}

/// 一条消息是不是画成的那一页：一个图片段，内容是那张图。
fn is_a_page(message: &[Value]) {
    assert_eq!(message.len(), 1, "{message:?}");
    assert_eq!(message[0]["type"], "image", "{message:?}");
    assert_eq!(
        message[0]["data"]["file"],
        format!("base64://{}", STANDARD.encode(PNG))
    );
}

/// 叫她一声。
fn call_her(napcat: &Answering) {
    napcat.send(group_frame(
        GROUP,
        ADMIN,
        1,
        json!([at(BOT), plain(" 讲讲")]),
        ("老板", "boss"),
    ));
}

#[tokio::test]
async fn a_long_reply_goes_out_as_pages_in_order() {
    let (dir, _) = temp_root();
    let record = dir.join("calls.jsonl");
    // 她先 @ 终端管理员（`mention`），再说长文：@ 只在第一页。
    let mention = format!(r#"{{"users":[{ADMIN}]}}"#);
    let script = Script::new([
        Play::calls(&[("mention", mention.as_str())]),
        Play::Says(long()),
    ]);
    let (home, mut napcat, _) = started_installed(&script, "", MEMBERS, renderer(&record)).await;
    call_her(&napcat);
    let first = napcat.group_message(GROUP).await;
    assert_eq!(
        first[0],
        json!({"type": "at", "data": {"qq": ADMIN.to_string()}}),
        "{first:?}"
    );
    is_a_page(&first[2..]);
    is_a_page(&napcat.group_message(GROUP).await);
    let events = until_events(&home.root, &format!("qq:group:{GROUP}"), |events| {
        of_kind(events, "venue.delivered").len() == 2
    })
    .await;
    let queued: Vec<Value> = of_kind(&events, "ext.onebot.venues.queued")
        .iter()
        .map(|one| one["body"]["kind"].clone())
        .collect();
    assert_eq!(queued, ["reply", "image"], "第一页算她的话");
    let delivered = of_kind(&events, "venue.delivered");
    assert_eq!(
        delivered[0]["body"]["text"],
        cleaned(),
        "第一页带全文（过了出站链的）"
    );
    assert_eq!(delivered[1]["body"]["text"], "");
    assert_eq!(
        delivered[0]["body"]["to"],
        json!([format!("qq:{ADMIN}")]),
        "第一页记回的人"
    );
    assert_eq!(delivered[1]["body"]["to"], json!([]));
    assert_eq!(
        delivered[0]["body"]["images"],
        delivered[1]["body"]["images"]
    );
    let calls = std::fs::read_to_string(&record).expect("记了");
    let call: Value = serde_json::from_str(calls.lines().next().expect("一次")).expect("JSON");
    assert_eq!(call["method"], "markdown");
    assert_eq!(
        call["params"]["markdown"],
        cleaned(),
        "渲染包收到的是过了出站链的全文"
    );
    assert_eq!(call["from"], "onebot");
    stopped(home).await;
}

#[tokio::test]
async fn a_short_reply_is_still_words() {
    let (dir, _) = temp_root();
    let record = dir.join("calls.jsonl");
    let script = Script::new([Play::Says("短的一句")]);
    let (home, mut napcat, _) = started_installed(&script, "", MEMBERS, renderer(&record)).await;
    call_her(&napcat);
    assert_eq!(napcat.group_reply(GROUP).await, "短的一句");
    assert!(
        !record.exists() || std::fs::read_to_string(&record).expect("读得出").is_empty(),
        "不调渲染包"
    );
    stopped(home).await;
}

#[tokio::test]
async fn without_the_renderer_a_long_reply_is_words() {
    let script = Script::new([Play::Says(long())]);
    let (home, mut napcat, _) =
        started_installed(&script, "", MEMBERS, |_: &miyu_store::root::DataRoot| {}).await;
    call_her(&napcat);
    assert_eq!(napcat.group_reply(GROUP).await, cleaned(), "画不成的照拆段");
    stopped(home).await;
}

#[tokio::test]
async fn a_group_set_to_split_gets_words() {
    let (dir, _) = temp_root();
    let record = dir.join("calls.jsonl");
    let script = Script::new([Play::Says(long())]);
    let rules = format!(
        "[[rule]]\nmatch = {{ kind = \"group\", group = [{GROUP}] }}\noutbound = {{ long_text = \"split\" }}\n"
    );
    let (home, mut napcat, _) =
        started_installed(&script, &rules, MEMBERS, renderer(&record)).await;
    call_her(&napcat);
    assert_eq!(napcat.group_reply(GROUP).await, cleaned());
    stopped(home).await;
}

#[tokio::test]
async fn a_renderer_that_draws_no_pages_does_not_swallow_the_reply() {
    let script = Script::new([Play::Says(long())]);
    let args = vec![r#"answer:markdown:{"pages":[]}"#.to_string()];
    let install =
        move |root: &miyu_store::root::DataRoot| install(root, "miyu-render", &args, &["markdown"]);
    let (home, mut napcat, _) = started_installed(&script, "", MEMBERS, install).await;
    call_her(&napcat);
    assert_eq!(napcat.group_reply(GROUP).await, cleaned(), "零页的照拆段");
    stopped(home).await;
}
