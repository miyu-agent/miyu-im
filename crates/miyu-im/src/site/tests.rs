use super::files::{kind, name};
use super::guard::{local_host, same_origin};
use super::logins::{HELD, Logins};

#[test]
fn only_local_hosts_on_the_listening_port_pass() {
    for host in ["127.0.0.1:8302", "localhost:8302", "[::1]:8302"] {
        assert!(local_host(Some(host), 8302), "{host}");
    }
    for host in [
        "127.0.0.1:8303",
        "evil.example:8302",
        "127.0.0.1",
        "localhost",
        "127.0.0.1:x",
        "127.0.0.2:8302",
        "[::1]",
        "",
    ] {
        assert!(!local_host(Some(host), 8302), "{host}");
    }
    assert!(!local_host(None, 8302), "没带 Host 的不认");
}

#[test]
fn an_origin_must_be_the_same_address() {
    assert!(same_origin(None, "127.0.0.1:8302"), "没带的不是浏览器发的");
    assert!(same_origin(Some("http://127.0.0.1:8302"), "127.0.0.1:8302"));
    for origin in [
        "http://127.0.0.1:8303",
        "https://127.0.0.1:8302",
        "http://localhost:8302",
        "null",
        "http://evil.example",
    ] {
        assert!(!same_origin(Some(origin), "127.0.0.1:8302"), "{origin}");
    }
}

#[test]
fn file_names_stay_in_the_directory() {
    assert_eq!(name("/"), Some("index.html"));
    assert_eq!(name("/app.js"), Some("app.js"));
    assert_eq!(name("/a-b_c.2.css"), Some("a-b_c.2.css"));
    assert_eq!(name("/shell/app.js"), Some("shell/app.js"));
    assert_eq!(name("/pages/home/home.js"), Some("pages/home/home.js"));
    assert_eq!(name("/a/b/c/d.js"), Some("a/b/c/d.js"));
    for path in [
        "/.hidden",
        "/../secret",
        "/a/../b.js",
        "/a/.git/config",
        "/a//b.js",
        "/a/",
        "/a/b/c/d/e.js",
        "/a%2Fb",
        "/a b",
        "app.js",
        "/..",
        "/a\\b",
    ] {
        assert_eq!(name(path), None, "{path}");
    }
}

#[test]
fn file_kinds_follow_the_extension() {
    assert_eq!(kind("index.html"), "text/html; charset=utf-8");
    assert_eq!(kind("app.js"), "text/javascript; charset=utf-8");
    assert_eq!(kind("style.css"), "text/css; charset=utf-8");
    assert_eq!(kind("a.json"), "application/json");
    assert_eq!(kind("a.svg"), "image/svg+xml");
    assert_eq!(kind("a.png"), "image/png");
    assert_eq!(kind("a.woff2"), "font/woff2");
    assert_eq!(kind("README"), "application/octet-stream");
    assert_eq!(kind("a.exe"), "application/octet-stream");
}

#[test]
fn logins_are_found_by_session_and_removed() {
    let logins = Logins::default();
    let a = logins.add("admin".to_string(), ()).expect("取得到随机数");
    let b = logins.add("mio".to_string(), ()).expect("取得到随机数");
    assert_eq!(a.len(), 64, "32 字节写成十六进制");
    assert!(a.chars().all(|c| c.is_ascii_hexdigit()), "{a}");
    assert_ne!(a, b, "各是各的会话号");
    assert_eq!(logins.get(&a).expect("在").account, "admin");
    assert_eq!(logins.get(&b).expect("在").account, "mio");
    assert!(logins.get(&a[..63]).is_none(), "短一位的不认");
    assert!(logins.get("").is_none());
    assert_eq!(logins.remove(&a).expect("有这一份").account, "admin");
    assert!(logins.remove(&a).is_none(), "拿掉过了");
    assert!(logins.get(&a).is_none());
    assert_eq!(logins.get(&b).expect("别的还在").account, "mio");
}

#[test]
fn only_the_latest_logins_are_held() {
    let logins = Logins::default();
    let sessions: Vec<String> = (0..=HELD)
        .map(|n| logins.add(n.to_string(), ()).expect("取得到随机数"))
        .collect();
    assert!(logins.get(&sessions[0]).is_none(), "最早的拿掉");
    assert_eq!(logins.get(&sessions[1]).expect("在").account, "1");
    assert_eq!(
        logins.get(&sessions[HELD]).expect("在").account,
        HELD.to_string()
    );
}

#[test]
fn log_lines_are_read_with_or_without_a_session() {
    use super::logs::parse;
    assert_eq!(
        parse(
            "2026-10-19 15:01:13.172 INFO  im       01a152bf-f0ca-71f2-ba98-721ab624c678 venue model pinned model=a/m"
        ),
        Some((
            "2026-10-19 15:01:13.172",
            "INFO",
            "venue model pinned model=a/m"
        ))
    );
    assert_eq!(
        parse("2026-10-19 15:01:13.172 WARN  im       reply not sent why=timeout"),
        Some((
            "2026-10-19 15:01:13.172",
            "WARN",
            "reply not sent why=timeout"
        ))
    );
    assert_eq!(
        parse("2026-10-19 15:01:13.172 INFO  im       abcdefghijabcdefghijabcdefghijabcdef rest"),
        Some((
            "2026-10-19 15:01:13.172",
            "INFO",
            "abcdefghijabcdefghijabcdefghijabcdef rest"
        )),
        "36 个字但不是会话编号的照留"
    );
    for broken in [
        "",
        "2026-10-19 15:01:13.172 LOUD  im x",
        "short",
        "2026-10-19 15:01:13.172 INFO  im       ",
    ] {
        assert_eq!(parse(broken), None, "{broken:?}");
    }
}

#[test]
fn recent_logs_come_newest_first_and_filter_by_level() {
    use super::logs::recent;
    let dir = std::env::temp_dir().join(format!("miyu-im-logs-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("建得了目录");
    let file = dir.join("im.log");
    std::fs::write(
        &file,
        "2026-10-19 10:00:00.000 INFO  im       first\n\
         2026-10-19 10:00:01.000 WARN  im       second\n\
         not a log line\n\
         2026-10-19 10:00:02.000 ERROR im       third\n\
         2026-10-19 10:00:03.000 INFO  im       fourth\n",
    )
    .expect("写得进");
    let texts = |filter: &str, limit: usize| -> Vec<String> {
        recent(&file, filter, limit)
            .iter()
            .map(|line| {
                format!(
                    "{} {} {}",
                    line["time"].as_str().unwrap_or_default(),
                    line["level"].as_str().unwrap_or_default(),
                    line["text"].as_str().unwrap_or_default()
                )
            })
            .collect()
    };
    assert_eq!(
        texts("all", 10),
        [
            "10:00:03 INFO fourth",
            "10:00:02 ERROR third",
            "10:00:01 WARN second",
            "10:00:00 INFO first"
        ]
    );
    assert_eq!(
        texts("warn", 10),
        ["10:00:02 ERROR third", "10:00:01 WARN second"]
    );
    assert_eq!(texts("error", 10), ["10:00:02 ERROR third"]);
    assert_eq!(
        texts("all", 2),
        ["10:00:03 INFO fourth", "10:00:02 ERROR third"]
    );
    assert_eq!(
        recent(&dir.join("none.log"), "all", 10),
        Vec::<serde_json::Value>::new()
    );
    if std::fs::remove_dir_all(&dir).is_err() {
        // 删不掉就留在临时目录里。
    }
}

#[test]
fn the_venue_page_gets_model_refs_pools_and_what_is_used() {
    // 施工 I-7 中一：选模型的下拉只要引用、用不了的标出来、池的名字、核心配的对话和看图模型。
    let listed = serde_json::json!({
        "providers": [
            {"id": "a", "models": [{"ref": "a/one", "state": "ok", "facts": {}}, {"ref": "a/two", "state": "no_key"}]},
            {"id": "b", "models": [{"ref": "b/three", "state": "cooling", "until": 1}, {"facts": {}}]},
            {"id": "c", "problem": "x"}
        ],
        "pools": [{"name": "快池", "models": ["a/one"]}, {"strategy": "x"}],
        "uses": {"chat": "a/one", "vision": null},
        "catalog": null
    });
    assert_eq!(
        super::venues::picked(&listed),
        serde_json::json!({
            "models": [{"ref": "a/one"}, {"ref": "a/two", "state": "no_key"}, {"ref": "b/three", "state": "cooling"}],
            "pools": ["快池"],
            "uses": {"chat": "a/one", "vision": null}
        })
    );
}

#[test]
fn the_venue_page_gets_presets_that_are_written_right() {
    // 施工 I-16：浮窗「预设」那一格只要编号、名字；没写名字的照编号，写错了的不列。
    let listed = serde_json::json!({"presets": [
        {"preset": "chat", "name": "聊天", "summary": "", "icon": null},
        {"preset": "plain", "name": "", "summary": ""},
        {"preset": "broken", "problem": "第 3 行写错了", "line": 3}
    ]});
    assert_eq!(
        super::venues::preset_choices(&listed),
        serde_json::json!({"presets": [{"id": "chat", "name": "聊天"}, {"id": "plain", "name": "plain"}]})
    );
}
