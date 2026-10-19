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
