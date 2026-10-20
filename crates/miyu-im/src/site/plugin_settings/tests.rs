//! 插件设置的几样纯逻辑（`super`）：最终值照写了的、默认、`null` 的先后；只改自己的键；密钥的名字照 WebUI 的写法起。

use serde_json::json;

use super::{all_own, secret_name, values};

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
        "dice",
        &[json!({"key": "dice.sides"}), json!({"key": "dice.a.b"})]
    ));
    assert!(!all_own(
        "dice",
        &[json!({"key": "dice.sides"}), json!({"key": "dicer.x"})]
    ));
    assert!(!all_own("dice", &[json!({"key": "dice"})]));
    assert!(!all_own("dice", &[json!({"value": 1})]));
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
