//! 几个驱动的配置各一份（施工 I-70 中上）：推来的变化只换、只通知那个平台的驱动；没挂的平台拿到空的。

use serde_json::{Map, Value, json};

use super::Current;
use crate::chat::Platform;
use crate::rules::Configured;

const QQ: Platform = Platform::new("qq");
const TG: Platform = Platform::new("tg");

/// `value` 当一份配置。
fn keys(value: Value) -> Map<String, Value> {
    value.as_object().expect("是对象").clone()
}

/// QQ、TG 各一个驱动，配置项都叫 `listen`，QQ 的默认 8301、TG 的没写默认值；握手交来 `config`。
fn two(config: Value) -> Current {
    Current::new(
        &config,
        vec![
            (QQ, &["listen"], keys(json!({"listen": 8301}))),
            (TG, &["listen"], Map::new()),
        ],
        Map::new(),
        Configured::default(),
    )
}

#[test]
fn each_driver_reads_its_own_group() {
    let current = two(json!({"im.qq.listen": 1, "im.tg.listen": 2}));
    assert_eq!(*current.driver(QQ).borrow(), keys(json!({"listen": 1})));
    assert_eq!(*current.driver(TG).borrow(), keys(json!({"listen": 2})));
    assert_eq!(current.driver_value(TG, "listen"), Some(json!(2)));
    assert_eq!(current.driver_default(QQ, "listen"), Some(json!(8301)));
    assert_eq!(
        current.driver_default(TG, "listen"),
        None,
        "默认值也各是各的"
    );
    let configs: Vec<_> = current.configs().into_iter().map(|(p, _)| p).collect();
    assert_eq!(configs, [QQ, TG], "照驱动的先后");
}

#[test]
fn a_push_notifies_only_the_platform_it_changes() {
    let current = two(json!({}));
    let (qq, mut tg) = (current.driver(QQ), current.driver(TG));
    current.change(&keys(json!({"im.tg.listen": 9})));
    assert!(tg.has_changed().expect("还在"), "TG 的变了");
    assert!(!qq.has_changed().expect("还在"), "QQ 的没变，不通知");
    assert_eq!(*tg.borrow_and_update(), keys(json!({"listen": 9})));
    current.change(&keys(json!({"im.qq.listen": null})));
    assert!(
        !qq.has_changed().expect("还在"),
        "回到默认值、和原来一样的不通知"
    );
    current.change(&keys(json!({"im.qq.listen": 7})));
    assert!(qq.has_changed().expect("还在"));
    assert!(!tg.has_changed().expect("还在"), "QQ 的变了不通知 TG");
}

#[test]
fn an_unmounted_platform_gets_nothing() {
    let current = two(json!({"im.wx.listen": 3}));
    let wx = Platform::new("wx");
    assert!(current.driver(wx).borrow().is_empty());
    assert_eq!(current.driver_value(wx, "listen"), None);
}
