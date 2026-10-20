//! 插件面板的票据和地址（`super`）：票据认得出是哪份登录、哪个插件，多了拿掉最早的；地址拆成三段、`%xx` 换回来、补上 `index.html`。

use super::{HELD, Panels, split};

#[test]
fn tickets_name_their_login_and_plugin_and_the_oldest_go_first() {
    let panels = Panels::default();
    let first = panels.open("session-a", "dice").expect("取得到随机数");
    let second = panels.open("session-b", "memes").expect("取得到随机数");
    assert_ne!(first, second);
    assert_eq!(
        panels.find(&first),
        Some(("session-a".into(), "dice".into()))
    );
    assert_eq!(
        panels.find(&second),
        Some(("session-b".into(), "memes".into()))
    );
    assert_eq!(panels.find("nope"), None);
    assert_eq!(panels.find(""), None, "空的对不上");
    assert_eq!(panels.find(&first[..16]), None, "只对上开头的不算");
    for _ in 0..HELD {
        panels.open("session-c", "dice");
    }
    assert_eq!(panels.find(&first), None, "最早的拿掉了");
}

#[test]
fn panel_paths_split_into_ticket_plugin_and_file() {
    let split_ok = |path: &str| split(path).expect("拆得开");
    assert_eq!(
        split_ok("/p/t1/dice/"),
        ("t1".into(), "dice".into(), "index.html".into())
    );
    assert_eq!(
        split_ok("/p/t1/dice"),
        ("t1".into(), "dice".into(), "index.html".into())
    );
    assert_eq!(
        split_ok("/p/t1/dice/js/app.js"),
        ("t1".into(), "dice".into(), "js/app.js".into())
    );
    assert_eq!(
        split_ok("/p/t1/dice/sub/"),
        ("t1".into(), "dice".into(), "sub/index.html".into())
    );
    assert_eq!(split_ok("/p/t1/dice/a%20b.css").2, "a b.css");
    assert_eq!(split_ok("/p/t1/dice/%E4%B8%AD.txt").2, "中.txt");
    for wrong in [
        "/q/t1/dice/",
        "/p//dice/",
        "/p/t1//x",
        "/p/t1",
        "/p/t1/dice/%zz",
        "/p/t1/dice/%E4",
    ] {
        assert_eq!(split(wrong), None, "{wrong}");
    }
}
