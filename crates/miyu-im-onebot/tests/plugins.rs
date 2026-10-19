//! im 的插件（施工 I-8 上，设计 01 第四节第 1 到 5 条）：真核心照开关拉起真桥，管理员家目录里装几个都导出 `chat.inbound` 的
//! 测试包。挂接点只问 im 的插件（带了 `plugin.toml`），在这个平台上能用的（`platforms`、`needs`），在这个场所开着的（场所卡片、
//! 兜底卡、声明的 `default`）。

use serde_json::{Value, json};

use miyu_session::testkit::Script;
use miyu_store::root::DataRoot;

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::hooked::{install, until_plugged};
use crate::support::*;

/// 群里的别人。
const LIN: i64 = 20004;

/// 群号。
const GROUP: i64 = 777;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 系统的场所规则：群都不抽样。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0 }\n";

/// 数据根里测试包 `id` 记调用的文件。
fn log_of(root: &DataRoot, id: &str) -> std::path::PathBuf {
    root.path().join(format!("{id}.calls"))
}

/// 装测试包 `id`：进站挂接点答 `pass`；`declared` 是它带给 im 的 `plugin.toml`，`None` 的不带（不是插件）。
fn plugin(root: &DataRoot, id: &str, declared: Option<&str>) {
    let record = format!("record:{}", log_of(root, id).display());
    let how = r#"answer:chat.inbound:{"route":"pass"}"#.to_string();
    install(root, id, &[record, how], &["chat.inbound"]);
    let dir = root.path().join("home/admin/packages").join(id);
    match declared {
        Some(declared) => {
            std::fs::write(dir.join("im").join("plugin.toml"), declared).expect("写得进");
        }
        None => {
            let manifest = dir.join("package.toml");
            let text = std::fs::read_to_string(&manifest).expect("读得出");
            let plain = text.replace("\n[content.im]\ndir = \"im\"\n", "");
            assert_ne!(plain, text, "真的去掉了");
            std::fs::write(&manifest, plain).expect("写得进");
        }
    }
}

/// 测试包 `id` 记下的调用。
fn calls(root: &DataRoot, id: &str) -> Vec<Value> {
    std::fs::read_to_string(log_of(root, id))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// 林在群里 @ 她，第 `message` 条。
fn lin_calls(message: i64) -> Value {
    group_frame(
        GROUP,
        LIN,
        message,
        json!([at(BOT), plain(" 在吗")]),
        ("小林", "lin"),
    )
}

/// 平台编号是 `message` 的那一条判下来了。
fn decided(events: &[Value], message: i64) -> bool {
    let cause = format!("qq:{BOT}:{message}:{TIME}/decided");
    events
        .iter()
        .any(|event| event["kind"] == "ext.im.chat.decided" && event["cause"] == cause)
}

#[tokio::test]
async fn only_plugins_usable_here_and_switched_on_are_asked() {
    let (home, napcat, _) = started_installed(&Script::new([]), RULES, MEMBERS, |root| {
        plugin(
            root,
            "good",
            Some("[plugin]\nname = { en = \"Good\" }\nneeds = [\"mute\"]\n"),
        );
        plugin(root, "plain", None);
        plugin(
            root,
            "elsewhere",
            Some("[plugin]\nname = { en = \"TG\" }\nplatforms = [\"tg\"]\n"),
        );
        plugin(
            root,
            "closed",
            Some("[plugin]\nname = { en = \"Off\" }\ndefault = false\n"),
        );
        plugin(
            root,
            "broken",
            Some("[plugin]\nname = { en = \"B\" }\nneeds = [\"fly\"]\n"),
        );
    })
    .await;
    // 五个都挂了，读得懂的插件三个（`plain` 不是插件，`broken` 写错了）。
    until_plugged(&home, 5, 3).await;
    let venue = format!("qq:group:{GROUP}");
    napcat.send(lin_calls(1));
    until_events(&home.root, &venue, |events| decided(events, 1)).await;
    assert_eq!(calls(&home.root, "good").len(), 1, "能用、开着的问了");
    for id in ["plain", "elsewhere", "closed", "broken"] {
        assert!(calls(&home.root, id).is_empty(), "{id} 不问");
    }
    let log = run_log(&home.root);
    assert!(
        log.contains("package=broken") && log.contains("plugin not read"),
        "{log}"
    );
    stopped(home).await;
}

#[tokio::test]
async fn a_saved_card_switches_a_plugin_on_or_off_in_its_venue_at_once() {
    let (home, napcat, _) = started_installed(&Script::new([]), RULES, MEMBERS, |root| {
        plugin(
            root,
            "closed",
            Some("[plugin]\nname = { en = \"Off\" }\ndefault = false\n"),
        );
        plugin(root, "open", Some("[plugin]\nname = { en = \"On\" }\n"));
    })
    .await;
    until_plugged(&home, 2, 2).await;
    // 后台页存卡片（施工 O-58 上）：这个群的卡上打开默认关着的、关掉默认开着的，当场生效。
    let mut head = Head::connect(&home.root).await;
    let cards = json!({
        "other_groups": {"on": true, "final_only": false},
        "groups": [{"id": GROUP, "set": {"plugins": {"closed": true, "open": false}}}],
    });
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let venue = format!("qq:group:{GROUP}");
    napcat.send(lin_calls(1));
    until_events(&home.root, &venue, |events| decided(events, 1)).await;
    assert_eq!(calls(&home.root, "closed").len(), 1, "卡上开了的问");
    assert!(calls(&home.root, "open").is_empty(), "卡上关了的不问");
    stopped(home).await;
}
