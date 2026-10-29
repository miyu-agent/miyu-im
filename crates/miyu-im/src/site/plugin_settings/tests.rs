//! 插件设置的几样纯逻辑（`super`）：最终值照写了的、默认、`null` 的先后；只改自己的键；密钥的名字照 WebUI 的写法起。

use serde_json::json;

use super::{Scope, all_own, own_items, secret_name, values};

#[test]
fn values_fall_back_to_the_default_then_null() {
    let items = [
        json!({"key": "dice.sides", "default": 6}),
        json!({"key": "dice.name"}),
        json!({"key": "dice.loud", "default": false}),
    ];
    let got = json!({"items": {"dice.loud": {"value": true, "origin": {}}}});
    assert_eq!(
        values(&items, &got),
        json!({"dice.sides": 6, "dice.name": null, "dice.loud": true})
    );
}

#[test]
fn only_own_keys_may_be_changed() {
    assert!(all_own(
        "dice.",
        &[json!({"key": "dice.sides"}), json!({"key": "dice.a.b"})]
    ));
    assert!(!all_own(
        "dice.",
        &[json!({"key": "dice.sides"}), json!({"key": "dicer.x"})]
    ));
    assert!(!all_own("dice.", &[json!({"key": "dice"})]));
    assert!(!all_own("dice.", &[json!({"value": 1})]));
    assert!(all_own(
        "im.qq.identity_",
        &[json!({"key": "im.qq.identity_people"})]
    ));
    assert!(!all_own(
        "im.qq.identity_",
        &[json!({"key": "im.qq.listen"})]
    ));
}

#[test]
fn a_builtin_plugin_lists_its_own_hidden_items_but_not_its_switch() {
    // 施工 I-10 上：驱动带的插件照键的开头找，总开关不列，藏起来的照列（拿掉 hidden）。
    let items = json!([
        {"key": "im.qq.identity_on", "package": "miyu-im", "hidden": true},
        {"key": "im.qq.identity_admin", "package": "miyu-im", "hidden": true},
        {"key": "im.qq.listen", "package": "miyu-im"},
        {"key": "dice.sides", "package": "dice"},
        // 核心给每个包加的「只对你自己关掉」（施工 I-38 换钉撞上的）：标着这个包，不是它自己的设置，不列。
        {"key": "packages.dice.off", "package": "dice"},
    ]);
    let builtin = Scope {
        package: "miyu-im-identity".into(),
        prefix: "im.qq.identity_".into(),
        builtin: true,
    };
    assert_eq!(
        own_items(&items, &builtin),
        [json!({"key": "im.qq.identity_admin", "package": "miyu-im"})]
    );
    let package = Scope {
        package: "dice".into(),
        prefix: "dice.".into(),
        builtin: false,
    };
    assert_eq!(
        own_items(&items, &package),
        [json!({"key": "dice.sides", "package": "dice"})]
    );
}

#[test]
fn secret_names_follow_the_key_and_the_time() {
    assert_eq!(secret_name("Dice.API_Key", 35), "dice-api-key-z");
    assert_eq!(secret_name("dice.token", 36), "dice-token-10");
    assert_eq!(
        secret_name("dice..x_y", 1),
        "dice-x-y-1",
        "连着的几个只换一个"
    );
    let long = format!("dice.{}", "a".repeat(100));
    assert_eq!(secret_name(&long, 0).chars().count(), 64);
}
