//! 订阅的回应和推送的先后（`super`）：回应晚于推送办完的，推来的不被盖掉；旧的回应不盖新的；没有订阅在等的推送不记。

use serde_json::{Value, json};

use super::*;

/// 总线上的一行：包 `package` 接进 `chat.hook`、登记了 `event`。
fn row(package: &str) -> Value {
    json!({"name": "chat.hook", "kind": "plugs", "package": package, "ready": true, "methods": ["event"]})
}

/// 整份名字：这几个包各一行。
fn names(packages: &[&str]) -> Value {
    Value::Array(packages.iter().map(|package| row(package)).collect())
}

fn hooked(followed: &Followed) -> Vec<&str> {
    followed.hooked.keys().map(String::as_str).collect()
}

#[test]
fn a_push_handled_before_the_answer_is_not_overwritten_by_it() {
    // 订阅那一刻 late 还没登记；登记了推来得早，回应后到：late 照样挂着。
    let mut followed = Followed::default();
    let ticket = followed.subscribing();
    followed.changed(&names(&["early", "late"]));
    assert!(followed.learn(ticket, &names(&["early"])));
    assert_eq!(hooked(&followed), ["early", "late"]);
    // 对上了以后推来的不再记。
    followed.changed(&names(&["early"]));
    assert!(followed.pushed.is_none());
    assert_eq!(hooked(&followed), ["early"]);
}

#[test]
fn an_answer_with_no_push_since_is_taken_whole() {
    let mut followed = Followed::default();
    let ticket = followed.subscribing();
    assert!(followed.learn(ticket, &names(&["a", "b"])));
    assert_eq!(hooked(&followed), ["a", "b"]);
}

#[test]
fn an_older_answer_never_replaces_a_newer_one() {
    // 掉队重新订阅：第二次的回应先到，第一次的后到也不要。
    let mut followed = Followed::default();
    let first = followed.subscribing();
    followed.changed(&names(&["a"]));
    let second = followed.subscribing();
    followed.changed(&names(&["a", "b"]));
    assert!(followed.learn(second, &names(&["a"])));
    assert_eq!(hooked(&followed), ["a", "b"]);
    assert!(!followed.learn(first, &names(&[])));
    assert_eq!(hooked(&followed), ["a", "b"]);
    assert!(followed.pushed.is_none());
}

#[test]
fn a_push_before_a_later_subscription_does_not_outlive_its_answer() {
    // 第一次订阅以后推来一份，再发第二次订阅（没有再推）：第一次的回应到了照推来的；第二次的回应是更晚那一刻的整份，照它。
    let mut followed = Followed::default();
    let first = followed.subscribing();
    followed.changed(&names(&["a"]));
    let second = followed.subscribing();
    assert!(followed.learn(first, &names(&[])));
    assert_eq!(hooked(&followed), ["a"]);
    assert!(followed.pushed.is_none());
    assert!(followed.learn(second, &names(&["b"])));
    assert_eq!(hooked(&followed), ["b"]);
}
