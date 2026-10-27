//! 插件自己的设置、插件面板调它的程序（施工 I-9 中）：真核心照开关拉起真桥和一个带两项设置的测试插件，页面经通道读、改它的设置（只改
//! 它自己的键，密钥先存成密钥再写引用），调它登记的方法；不是 im 插件的不收。

use serde_json::{Value, json};

use miyu_session::testkit::Script;
use miyu_store::root::DataRoot;

use crate::support::browser::{call, logged_in};
use crate::support::group::*;
use crate::support::head::Head;
use crate::support::hooked::{install, until_hooked};
use crate::support::*;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 骰子插件的两项设置。
const SETTINGS: &str = r#"
[settings.sides]
type = "int"
min = 2
max = 100
default = 6
layers = ["system"]
applies = "now"
name = { en = "Sides", zh = "面数" }
description = { en = "Sides of the die." }

[settings.greeting]
type = "text"
layers = ["system"]
applies = "now"
name = { en = "Greeting" }
description = { en = "What it says." }

[settings.token]
type = "secret"
layers = ["system"]
applies = "now"
name = { en = "Token" }
description = { en = "The token." }
"#;

/// 装骰子插件：登记 `dice.roll`（答 `{"rolled": 4}`），清单里多两项设置。
fn dice(root: &DataRoot) {
    let roll = r#"answer:dice.roll:{"rolled":4}"#.to_string();
    let record = format!("record:{}", root.path().join("dice.calls").display());
    install(root, "dice", &[record, roll], &["chat.inbound"]);
    let manifest = root.path().join("system/packages/dice/package.toml");
    let text = std::fs::read_to_string(&manifest).expect("读得出");
    std::fs::write(&manifest, format!("{text}{SETTINGS}")).expect("写得进");
}

/// `plugins.settings.set` 的参数。
fn changes(package: &str, changes: Value) -> Value {
    json!({"package": package, "changes": changes})
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_s_own_settings_and_methods_are_reached_from_the_page() {
    let (home, _napcat, _) = started_installed(&Script::new([]), "", MEMBERS, dice).await;
    let mut head = Head::connect(&home.root).await;
    let status = head.until("status", |status| status["web"].is_u64()).await;
    let web = u16::try_from(status["web"].as_u64().expect("有端口")).expect("是端口");
    let mut open = logged_in(&home.root, web).await;
    // 桥读到骰子插件以前问的回 `bad_params`（不是 im 插件）：先等它读到（Windows 跑得慢时撞上过）。
    until_hooked(&home, 1).await;

    let got = call(
        &mut open,
        1,
        "plugins.settings.get",
        json!({"package": "dice"}),
    )
    .await;
    let keys: Vec<&Value> = got["result"]["items"]
        .as_array()
        .expect("有项")
        .iter()
        .map(|item| &item["key"])
        .collect();
    assert_eq!(
        keys,
        [
            &json!("dice.sides"),
            &json!("dice.greeting"),
            &json!("dice.token")
        ],
        "{got}"
    );
    assert_eq!(
        got["result"]["items"][0]["name"], "面数",
        "照连接的语言：{got}"
    );
    assert_eq!(
        got["result"]["values"],
        json!({"dice.sides": 6, "dice.greeting": null, "dice.token": null})
    );

    let both = json!([
        {"key": "dice.sides", "value": 20},
        {"key": "dice.greeting", "value": "hi"},
        {"key": "dice.token", "value": "abc"},
    ]);
    let saved = call(&mut open, 2, "plugins.settings.set", changes("dice", both)).await;
    assert!(saved["result"]["keys"]["dice.sides"].is_object(), "{saved}");
    let got = call(
        &mut open,
        3,
        "plugins.settings.get",
        json!({"package": "dice"}),
    )
    .await;
    let values = &got["result"]["values"];
    assert_eq!(values["dice.sides"], 20, "{got}");
    assert_eq!(values["dice.greeting"], "hi", "不是密钥的照写：{got}");
    let secret = values["dice.token"]["secret"].as_str().unwrap_or_default();
    assert!(secret.starts_with("dice-token-"), "写成引用：{got}");

    // 恢复默认；别的键、别的包、写错的不收。
    let reset = json!([{"key": "dice.sides", "unset": true}]);
    call(&mut open, 4, "plugins.settings.set", changes("dice", reset)).await;
    let got = call(
        &mut open,
        5,
        "plugins.settings.get",
        json!({"package": "dice"}),
    )
    .await;
    assert_eq!(got["result"]["values"]["dice.sides"], 6, "{got}");
    let theirs =
        json!([{"key": "dice.sides", "value": 3}, {"key": "im.strip_period", "value": false}]);
    let refused = call(
        &mut open,
        6,
        "plugins.settings.set",
        changes("dice", theirs),
    )
    .await;
    assert_eq!(refused["error"]["reason"], "forbidden", "{refused}");
    let wrongs = [
        changes("im", json!([{"key": "im.strip_period", "value": false}])),
        changes("dice", json!([])),
        json!({"package": "dice"}),
    ];
    for wrong in wrongs {
        let refused = call(&mut open, 7, "plugins.settings.set", wrong.clone()).await;
        assert_eq!(
            refused["error"]["reason"], "bad_params",
            "{wrong} {refused}"
        );
    }
    let invalid = json!([{"key": "dice.sides", "value": 1000}]);
    let refused = call(
        &mut open,
        8,
        "plugins.settings.set",
        changes("dice", invalid),
    )
    .await;
    assert_eq!(refused["error"]["reason"], "config_invalid", "{refused}");

    // 调它登记的方法。
    let roll = json!({"package": "dice", "method": "dice.roll", "params": {"n": 1}});
    let rolled = call(&mut open, 9, "plugins.call", roll).await;
    assert_eq!(rolled["result"], json!({"rolled": 4}), "{rolled}");
    let calls = std::fs::read_to_string(home.root.path().join("dice.calls")).unwrap_or_default();
    let last: Value =
        serde_json::from_str(calls.lines().last().unwrap_or("null")).expect("是 JSON");
    assert_eq!(last["params"], json!({"n": 1}), "参数照传：{calls}");
    let other = json!({"package": "im", "method": "status"});
    let refused = call(&mut open, 10, "plugins.call", other).await;
    assert_eq!(refused["error"]["reason"], "bad_params", "{refused}");
    stopped(home).await;
}
