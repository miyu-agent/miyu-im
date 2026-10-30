//! im 的插件（施工 I-8 上，设计 01 第四节第 1 到 5 条）：真核心照开关拉起真桥，管理员家目录里装几个都导出 `chat.inbound` 的
//! 测试包。挂接点只问 im 的插件（带了 `plugin.toml`），在这个平台上能用的（`platforms`、`needs`），在这个场所开着的（场所卡片、
//! 兜底卡、声明的 `default`）。插件经 im 导出的 `chat.note` 往场所会话里记一句给 AI 的事实（第 6 条）。

use serde_json::{Value, json};

use miyu_session::testkit::Script;
use miyu_store::root::DataRoot;

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::hooked::{approve, install, linked, until_plugged};
use crate::support::*;

/// 群里的别人。
const LIN: i64 = 20004;

/// 群号。
const GROUP: i64 = 777;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 系统的场所规则：只有一条配群的空规则。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\n";

/// 数据根里测试包 `id` 记调用的文件。
fn log_of(root: &DataRoot, id: &str) -> std::path::PathBuf {
    root.path().join(format!("{id}.calls"))
}

/// 装测试包 `id`：进站挂接点答 `pass`；`declared` 是它带给 im 的 `plugin.toml`，`None` 的不带（不是插件）。
fn plugin(root: &DataRoot, id: &str, declared: Option<&str>) {
    let record = format!("record:{}", log_of(root, id).display());
    let how = r#"answer:chat.inbound:{"route":"pass"}"#.to_string();
    install(root, id, &[record, how], &["chat.inbound"]);
    let dir = root.path().join("packages").join(id);
    match declared {
        Some(declared) => {
            std::fs::write(dir.join("im").join("plugin.toml"), declared).expect("写得进");
        }
        None => {
            let manifest = dir.join("package.toml");
            let text = std::fs::read_to_string(&manifest).expect("读得出");
            let plain = text.replace("\n[content.miyu-im]\ndir = \"im\"\n", "");
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
    let venue = format!("qq:30003:group:{GROUP}");
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
    // 会话浮窗「管理插件」（施工 I-7 中四）：`cards.get` 列出认得的插件，用不了的写为什么。
    let mut head = Head::connect(&home.root).await;
    let got = head.page_call("cards.get").await["result"]["plugins"].clone();
    let find = |id: &str| {
        got.as_array()
            .and_then(|all| all.iter().find(|one| one["package"] == id).cloned())
            .unwrap_or_else(|| panic!("没有 {id}：{got}"))
    };
    assert_eq!(
        find("good"),
        json!({"package": "good", "name": {"en": "Good"}, "summary": {}, "default": true, "needs": ["mute"]})
    );
    assert_eq!(find("elsewhere")["unusable"], "other_platforms", "{got}");
    assert_eq!(find("closed")["default"], false, "{got}");
    assert_eq!(
        find("broken"),
        json!({"package": "broken", "unusable": "broken"})
    );
    assert!(
        got.as_array()
            .is_some_and(|all| all.iter().all(|one| one["package"] != "plain")),
        "不是插件的不列：{got}"
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
    let venue = format!("qq:30003:group:{GROUP}");
    napcat.send(lin_calls(1));
    until_events(&home.root, &venue, |events| decided(events, 1)).await;
    assert_eq!(calls(&home.root, "closed").len(), 1, "卡上开了的问");
    assert!(calls(&home.root, "open").is_empty(), "卡上关了的不问");
    stopped(home).await;
}

#[tokio::test]
async fn a_plugin_notes_a_fact_into_the_venue_session() {
    let (home, napcat, _) = started_installed(&Script::new([]), RULES, MEMBERS, |root| {
        let record = format!("record:{}", log_of(root, "admin").display());
        let note = "note:moderated:小周 was muted by 阿杰 for 10 minutes.".to_string();
        let how = r#"answer:chat.inbound:{"route":"pass"}"#.to_string();
        install(root, "admin", &[record, note, how], &["chat.inbound"]);
        approve(root, "admin", &["packages.call"]);
    })
    .await;
    until_plugged(&home, 1, 1).await;
    let venue = format!("qq:30003:group:{GROUP}");
    napcat.send(lin_calls(1));
    let events = until_events(&home.root, &venue, |events| {
        events
            .iter()
            .any(|event| event["body"]["kind"] == "moderated")
    })
    .await;
    let fact = events
        .iter()
        .find(|event| event["body"]["kind"] == "moderated")
        .expect("记了");
    assert_eq!(fact["kind"], "context.injected", "{fact}");
    assert_eq!(
        fact["body"]["text"].as_str().map(str::trim_end),
        Some("<moderated>小周 was muted by 阿杰 for 10 minutes.</moderated>"),
        "套上外壳：种类当标签"
    );
    // 事实先记进会话，测试包收到回应、记下来要晚一点：等它记下。
    let deadline = tokio::time::Instant::now() + WAIT;
    let noted = loop {
        let noted: Vec<Value> = calls(&home.root, "admin")
            .into_iter()
            .filter_map(|line| line.get("noted").cloned())
            .collect();
        if !noted.is_empty() {
            break noted;
        }
        assert!(tokio::time::Instant::now() < deadline, "测试包没记下回应");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    assert_eq!(noted.len(), 1, "{noted:?}");
    assert_eq!(noted[0]["result"], json!({"noted": true}), "{noted:?}");
    assert!(run_log(&home.root).contains("plugin noted"));
    stopped(home).await;
}

#[tokio::test]
async fn a_note_where_the_plugin_is_off_or_there_is_no_session_is_refused() {
    let (home, napcat, _) = started_installed(&Script::new([]), RULES, MEMBERS, |root| {
        let record = format!("record:{}", log_of(root, "admin").display());
        let args = [
            record,
            "note:moderated:x".to_string(),
            format!("note-at:qq:30003:group:{GROUP}"),
            "note-at:qq:30003:group:888".to_string(),
            "note-at:qq:30003:group:999".to_string(),
            "note-at:qq:nowhere".to_string(),
            r#"answer:chat.inbound:{"route":"pass"}"#.to_string(),
        ];
        install(root, "admin", &args, &["chat.inbound"]);
        approve(root, "admin", &["packages.call"]);
    })
    .await;
    until_plugged(&home, 1, 1).await;
    // 888 这个群的卡上关了它（还要让陌生群接话：存卡片是整份换）。
    let mut head = Head::connect(&home.root).await;
    let cards = json!({
        "other_groups": {"on": true, "final_only": false},
        "groups": [{"id": 888, "set": {"plugins": {"admin": false}}}],
    });
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    napcat.send(lin_calls(1));
    let deadline = tokio::time::Instant::now() + WAIT;
    let noted = loop {
        let noted: Vec<Value> = calls(&home.root, "admin")
            .into_iter()
            .filter(|line| line.get("noted").is_some())
            .collect();
        if noted.len() >= 4 {
            break noted;
        }
        assert!(tokio::time::Instant::now() < deadline, "记了 {noted:?}");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    let answer = |venue: &str| {
        let line = noted
            .iter()
            .find(|line| line["venue"] == venue)
            .expect("记了这一次");
        line["noted"].clone()
    };
    assert_eq!(
        answer(&format!("qq:30003:group:{GROUP}"))["result"],
        json!({"noted": true})
    );
    // 核心转回来的拒：原因在 `error.data.message`。
    let refused = |venue: &str| answer(venue)["error"]["data"]["message"].clone();
    assert_eq!(refused("qq:30003:group:888"), "not_allowed", "卡上关了它");
    assert_eq!(
        refused("qq:30003:group:999"),
        "no_session",
        "开着、还没有会话"
    );
    assert_eq!(refused("qq:nowhere"), "bad_params", "不是场所");
    stopped(home).await;
}

/// 一条群成员进群的通知：群 `group`、进来的是 `user`。
fn joined(group: i64, user: i64) -> Value {
    json!({"post_type": "notice", "notice_type": "group_increase", "sub_type": "approve", "time": TIME, "self_id": BOT, "group_id": group, "user_id": user, "operator_id": 0})
}

#[tokio::test]
async fn a_platform_event_goes_only_to_plugins_on_in_its_venue() {
    let (home, napcat, _) = started_installed(&Script::new([]), RULES, MEMBERS, |root| {
        let record = format!("record:{}", log_of(root, "watcher").display());
        install(
            root,
            "watcher",
            &[record, "answer:chat.event:{}".to_string()],
            &["chat.event"],
        );
    })
    .await;
    until_plugged(&home, 1, 1).await;
    let mut head = Head::connect(&home.root).await;
    let cards = json!({
        "other_groups": {"on": true, "final_only": false},
        "groups": [{"id": 888, "set": {"plugins": {"watcher": false}}}],
    });
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    // 关着它的群先来一条，开着的群后来一条：后一条到了，前一条要是交了也早到了。
    napcat.send(joined(888, 5));
    napcat.send(joined(GROUP, 6));
    let deadline = tokio::time::Instant::now() + WAIT;
    while !calls(&home.root, "watcher")
        .iter()
        .any(|call| call["params"]["event"]["user_id"] == 6)
    {
        assert!(
            tokio::time::Instant::now() < deadline,
            "开着的群的通知一直没到"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(
        calls(&home.root, "watcher")
            .iter()
            .all(|call| call["params"]["event"]["group_id"] != 888),
        "关着它的群的通知不交"
    );
    stopped(home).await;
}

#[tokio::test]
async fn a_plugin_installed_while_running_is_known_from_the_pushed_change() {
    let (home, _napcat, _) = started_installed(&Script::new([]), RULES, MEMBERS, |_| {}).await;
    until_plugged(&home, 0, 0).await;
    // 跑着的时候装一个带 `plugin.toml` 的包：核心重读清单、推 `content.changed`，im 再读一遍插件。
    let dir = home.root.path().join("staging").join("late");
    std::fs::create_dir_all(dir.join("im")).expect("建得了目录");
    let manifest = format!(
        "[package]\nprotocol = [1, 1]\nname = {{ en = \"Late\" }}\n\n[command]\nname = \"late\"\nprogram = \"{}\"\nabout = {{ en = \"L\" }}\n\n[process]\nargs = []\nstart = \"manual\"\n\n[content.miyu-im]\ndir = \"im\"\n",
        linked()
    );
    std::fs::write(dir.join("package.toml"), manifest).expect("写得进");
    std::fs::write(
        dir.join("im").join("plugin.toml"),
        "[plugin]\nname = { en = \"Late\" }\n",
    )
    .expect("写得进");
    let mut head = Head::connect(&home.root).await;
    let installed = head
        .call(
            "package.install",
            json!({"path": dir.display().to_string()}),
        )
        .await;
    assert!(installed.get("result").is_some(), "{installed}");
    until_plugged(&home, 0, 1).await;
    assert!(run_log(&home.root).contains("plugin known package=late"));
    stopped(home).await;
}
