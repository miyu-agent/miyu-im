//! 私聊默认值（施工 I-112）。

use serde_json::json;

use super::*;

fn cards(value: serde_json::Value) -> Cards {
    serde_json::from_value(value).expect("写法对")
}

#[test]
fn old_cards_keep_private_chats_on_the_old_defaults() {
    // I-112 以前的 cards.json：只有一张会话默认值。
    let old = cards(
        json!({"defaults": {"model": "dev/m", "sleep": "01:00-08:00", "keywords": ["miyu"], "glance": true}}),
    );
    assert!(old.private_defaults.is_none());
    let file = old.private_defaults_file();
    assert!(file.text.contains("kind = \"private\""), "{}", file.text);
    assert_eq!(
        file.text.replace("kind = \"private\"", ""),
        old.defaults_file().text.replace("kind = \"group\"", ""),
        "没写过的私聊默认值照原来的会话默认值那几行，私聊照旧"
    );
    let set = old.private_set();
    assert_eq!(set.model.as_deref(), Some("dev/m"));
    assert_eq!(set.glance, Some(true));
    assert_eq!(
        set.rules.expect("搬了睡觉时间")["sleep"],
        json!("01:00-08:00")
    );
}

#[test]
fn written_private_defaults_only_apply_to_private_chats() {
    let split = cards(json!({
        "defaults": {"glance": true},
        "private_defaults": {"glance": false, "model": "dev/p"},
    }));
    let group = split.defaults_file();
    let private = split.private_defaults_file();
    assert!(group.text.contains("kind = \"group\""), "{}", group.text);
    assert!(
        private.text.contains("kind = \"private\""),
        "{}",
        private.text
    );
    assert!(private.text.contains("dev/p"), "{}", private.text);
    assert!(
        !private.text.contains("glance = true"),
        "群聊的看一眼不套私聊：{}",
        private.text
    );
    assert_eq!(Card::parse(&private.name), Some(Card::PrivateDefaults));
    assert_eq!(Card::parse(&group.name), Some(Card::Defaults));
}

#[test]
fn nothing_written_means_no_private_file() {
    assert!(Cards::default().private_defaults_unset());
    assert!(
        !cards(json!({"defaults": {"glance": true}})).private_defaults_unset(),
        "照群聊默认值"
    );
    assert!(
        cards(json!({"defaults": {"glance": true}, "private_defaults": {}}))
            .private_defaults_unset(),
        "写过、清空了的"
    );
}
