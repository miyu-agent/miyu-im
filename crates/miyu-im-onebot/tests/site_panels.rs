//! 插件面板（施工 I-9 下）：真核心照开关拉起真桥和一个带后台页的测试插件，页面要一张票据，照它取框里的文件（照 `package.file`，
//! 沙箱的回应头，`Origin: null` 也收）；票据、插件对不上的 404，没有后台页的 `no_page`，登出以后票据没用了。

use serde_json::json;

use miyu_session::testkit::Script;
use miyu_store::root::DataRoot;

use crate::support::browser::{call, channel, heard, post, request, say};
use crate::support::group::*;
use crate::support::head::Head;
use crate::support::hooked::{install, until_plugged};
use crate::support::*;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 后台页的入口。
const INDEX: &str = "<!doctype html><script type=module src=app.js></script>";

/// 装三个插件：`dice`、`board` 带后台页（`page/index.html`、`page/js/app.js`），`plain` 不带。
fn plugins(root: &DataRoot) {
    install(root, "plain", &[], &["chat.inbound"]);
    for id in ["dice", "board"] {
        install(root, id, &[], &["chat.inbound"]);
        let dir = root.path().join("packages").join(id);
        std::fs::create_dir_all(dir.join("page/js")).expect("建得了目录");
        std::fs::write(dir.join("page/index.html"), INDEX).expect("写得进");
        std::fs::write(dir.join("page/js/app.js"), "export const x = 1;\n").expect("写得进");
        let manifest = dir.join("package.toml");
        let text = std::fs::read_to_string(&manifest).expect("读得出");
        std::fs::write(&manifest, format!("{text}\n[page]\ndir = \"page\"\n")).expect("写得进");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_panel_is_served_in_a_sandbox_by_ticket() {
    let (home, _napcat, _) = started_installed(&Script::new([]), "", MEMBERS, plugins).await;
    // 桥读到三个插件以后才认得它们（I-24：CI 的 Windows 上没等，`dice` 还不认得，回的是错）。
    until_plugged(&home, 3, 3).await;
    let mut head = Head::connect(&home.root).await;
    let status = head.until("status", |status| status["web"].is_u64()).await;
    let web = u16::try_from(status["web"].as_u64().expect("有端口")).expect("是端口");
    // 照页面的样子登录，留着会话号（后面登出）。
    let ticket = head.call("login.ticket", json!({})).await["result"]["ticket"].clone();
    let session = post(web, "/login/ticket", &[], &json!({"ticket": ticket}))
        .await
        .json()["session"]
        .as_str()
        .expect("登录了")
        .to_string();
    let mut open = channel(web, None).await;
    say(&mut open, &json!({"session": session})).await;
    assert!(heard(&mut open).await.is_ok());

    let opened = call(&mut open, 1, "plugins.panel", json!({"package": "dice"})).await;
    let url = opened["result"]["url"]
        .as_str()
        .unwrap_or_else(|| panic!("有地址：{opened}"))
        .to_string();
    assert!(
        url.starts_with("/p/") && url.ends_with("/dice/"),
        "{opened}"
    );
    let host = format!("127.0.0.1:{web}");
    let index = request(web, ("GET", &url, &host), &["Origin: null"], b"").await;
    assert_eq!(index.status, 200);
    assert_eq!(String::from_utf8_lossy(&index.body), INDEX);
    assert_eq!(
        index.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    let policy = index.header("content-security-policy").unwrap_or_default();
    assert!(
        policy.starts_with("sandbox allow-scripts allow-forms;"),
        "{policy}"
    );
    assert!(policy.contains("connect-src 'none'"), "{policy}");
    assert_eq!(index.header("access-control-allow-origin"), Some("*"));
    assert_eq!(index.header("referrer-policy"), Some("no-referrer"));
    let script = request(
        web,
        ("GET", &format!("{url}js/app.js"), &host),
        &["Origin: null"],
        b"",
    )
    .await;
    assert_eq!(script.status, 200);
    assert_eq!(
        script.header("content-type"),
        Some("text/javascript; charset=utf-8")
    );

    // 对不上的：别的插件、没有的文件、编出来的票据、别的网站。
    // 别的插件有后台页也不行：票据是给 dice 的。
    let other = url.replace("/dice/", "/board/");
    let missing = format!("{url}nope.js");
    let forged = format!("/p/{}/dice/", "0".repeat(64));
    for path in [other.as_str(), missing.as_str(), forged.as_str()] {
        let answer = request(web, ("GET", path, &host), &[], b"").await;
        assert_eq!(answer.status, 404, "{path}");
    }
    let elsewhere = request(
        web,
        ("GET", &url, &host),
        &["Origin: http://evil.invalid"],
        b"",
    )
    .await;
    assert_eq!(elsewhere.status, 403);
    let none = call(&mut open, 2, "plugins.panel", json!({"package": "plain"})).await;
    assert_eq!(none["error"]["reason"], "no_page", "{none}");
    let wrong = call(&mut open, 3, "plugins.panel", json!({"package": "miyu-im"})).await;
    assert_eq!(wrong["error"]["reason"], "bad_params", "{wrong}");

    // 登出以后票据没用了。
    let bearer = format!("Authorization: Bearer {session}");
    assert_eq!(
        post(web, "/logout", &[&bearer], &json!({})).await.status,
        204
    );
    let after = request(web, ("GET", &url, &host), &[], b"").await;
    assert_eq!(after.status, 404);
    stopped(home).await;
}
