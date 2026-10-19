//! 订阅的回应和推送的先后（`super`）：回应晚于推送办完的，推来的不被盖掉；旧的回应不盖新的；没有订阅在等的推送不记。

use serde_json::{Value, json};

use super::*;

/// 包 `package` 的一项：在跑的导出 `chat.event`，不在跑的什么都不挂。
fn entry(package: &str, running: bool) -> Value {
    let status = if running { "running" } else { "disabled" };
    json!({"package": package, "status": status, "export": {"methods": ["chat.event"]}})
}

fn hooked(followed: &Followed) -> Vec<&str> {
    followed.hooked.keys().map(String::as_str).collect()
}

#[test]
fn a_push_handled_before_the_answer_is_not_overwritten_by_it() {
    // 订阅那一刻 late 关着；打开推来得早，回应后到：late 照样挂着。
    let mut followed = Followed::default();
    let ticket = followed.subscribing();
    followed.changed("late", &entry("late", true));
    let snapshot = json!([entry("early", true), entry("late", false)]);
    assert!(followed.learn(ticket, &snapshot));
    assert_eq!(hooked(&followed), ["early", "late"]);
    // 对上了以后推来的不再记。
    followed.changed("late", &entry("late", false));
    assert!(followed.pushed.is_empty());
    assert_eq!(hooked(&followed), ["early"]);
}

#[test]
fn an_older_answer_never_replaces_a_newer_one() {
    // 掉队重新订阅：第二次的回应先到，第一次的后到也不要。
    let mut followed = Followed::default();
    let first = followed.subscribing();
    followed.changed("a", &entry("a", true));
    let second = followed.subscribing();
    followed.changed("b", &entry("b", true));
    assert!(followed.learn(second, &json!([entry("a", true)])));
    assert_eq!(hooked(&followed), ["a", "b"]);
    assert!(!followed.learn(first, &json!([])));
    assert_eq!(hooked(&followed), ["a", "b"]);
    assert!(followed.pushed.is_empty());
}

#[test]
fn pushes_before_a_later_subscription_are_kept_only_for_it_when_needed() {
    // 第一次的回应先到：第二次订阅以后推来的留着，等第二次的回应再办一遍。
    let mut followed = Followed::default();
    let first = followed.subscribing();
    followed.changed("a", &entry("a", true));
    let second = followed.subscribing();
    followed.changed("b", &entry("b", true));
    assert!(followed.learn(first, &json!([])));
    assert_eq!(hooked(&followed), ["a", "b"]);
    assert_eq!(followed.pushed.len(), 1);
    assert!(followed.learn(second, &json!([])));
    assert_eq!(hooked(&followed), ["b"]);
    assert!(followed.pushed.is_empty());
}
