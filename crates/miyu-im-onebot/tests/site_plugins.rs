//! im 页面的插件页（施工 I-9 上）：真核心照开关拉起真桥和两个测试插件，页面登录以后经通道读插件（im 认得的并上核心说的版本、开关、
//! 挂在哪）、读 README（经桥自己的连接读插件带给 im 的目录）、开关插件（人的连接上 `package.enable`、`package.disable`）。

use serde_json::{Value, json};

use miyu_session::testkit::Script;
use miyu_store::root::DataRoot;

use crate::support::browser::{call, logged_in};
use crate::support::group::*;
use crate::support::head::Head;
use crate::support::hooked::{install, install_starting, until_plugged};
use crate::support::*;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 骰子插件带的 README。
const README: &str = "# 骰子\n\n掷骰命令 `.r 1d100`。\n";

/// 装两个插件：`dice` 一起来就拉起、声明写全、带 README；`lazy` 要开关打开才拉起、只写名字。
fn plugins(root: &DataRoot) {
    install(root, "dice", &[], &["chat.inbound"]);
    let dir = root.path().join("packages/dice/im");
    let declared = "[plugin]\nname = { zh = \"骰子\", en = \"Dice\" }\nsummary = { zh = \"掷骰\" }\nicon = \"dice-5\"\nhomepage = \"https://dice.invalid\"\nreadme = \"README.md\"\n";
    std::fs::write(dir.join("plugin.toml"), declared).expect("写得进");
    std::fs::write(dir.join("README.md"), README).expect("写得进");
    install_starting(root, "lazy", &[], &["chat.event"], "manual");
}

/// 插件页列出来的 `package` 那一个。
fn one<'a>(listed: &'a Value, package: &str) -> &'a Value {
    listed["result"]["plugins"]
        .as_array()
        .and_then(|all| all.iter().find(|plugin| plugin["package"] == package))
        .unwrap_or_else(|| panic!("列出了 {package}：{listed}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn the_plugin_page_lists_reads_and_switches_plugins_through_the_core() {
    let (home, _napcat, _) = started_installed(&Script::new([]), "", MEMBERS, plugins).await;
    let mut head = Head::connect(&home.root).await;
    let status = head.until("status", |status| status["web"].is_u64()).await;
    let web = u16::try_from(status["web"].as_u64().expect("有端口")).expect("是端口");
    let mut open = logged_in(&home.root, web).await;
    // 桥从软件包列表的订阅里读到两个插件要一会儿（Windows 上撞上过：列出来只有驱动带的）：先等挂上一个、读到两个。
    until_plugged(&home, 1, 2).await;

    let listed = call(&mut open, 1, "plugins.list", json!({})).await;
    let dice = one(&listed, "dice");
    assert_eq!(dice["name"], json!({"zh": "骰子", "en": "Dice"}), "{dice}");
    assert_eq!(dice["icon"], "dice-5", "{dice}");
    assert_eq!(dice["links"]["readme"], "README.md", "{dice}");
    assert_eq!(dice["enabled"], true, "{dice}");
    assert_eq!(dice["points"], json!(["chat.inbound"]), "{dice}");
    assert_eq!(dice["from"], "local", "{dice}");
    let lazy = one(&listed, "lazy");
    assert_eq!(lazy["enabled"], false, "{lazy}");
    assert!(lazy.get("links").is_none(), "{lazy}");

    let read = call(&mut open, 2, "plugins.readme", json!({"package": "dice"})).await;
    assert_eq!(read["result"]["text"], README, "{read}");
    let none = call(&mut open, 3, "plugins.readme", json!({"package": "lazy"})).await;
    assert_eq!(none["error"]["reason"], "not_found", "{none}");

    // 开一个、关一个；不是 im 插件的不让动。
    let on = json!({"package": "lazy", "on": true});
    let switched = call(&mut open, 4, "plugins.enable", on).await;
    assert_eq!(switched["result"]["enabled"], true, "{switched}");
    let off = json!({"package": "dice", "on": false});
    let switched = call(&mut open, 5, "plugins.enable", off).await;
    assert_eq!(switched["result"]["enabled"], false, "{switched}");
    let listed = call(&mut open, 6, "plugins.list", json!({})).await;
    assert_eq!(one(&listed, "lazy")["enabled"], true, "{listed}");
    assert_eq!(one(&listed, "dice")["enabled"], false, "{listed}");
    let wrongs = [
        json!({"package": "miyu-im", "on": false}),
        json!({"package": "dice"}),
        json!({"package": "lazy", "on": true, "approve": true}),
        json!({"on": true}),
    ];
    for wrong in wrongs {
        let refused = call(&mut open, 7, "plugins.enable", wrong.clone()).await;
        assert_eq!(
            refused["error"]["reason"], "bad_params",
            "{wrong} {refused}"
        );
    }
    let unknown = call(
        &mut open,
        8,
        "plugins.readme",
        json!({"package": "miyu-im"}),
    )
    .await;
    assert_eq!(unknown["error"]["reason"], "bad_params", "{unknown}");
    stopped(home).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_builtin_identity_plugin_is_listed_switched_and_set_from_the_page() {
    // 施工 I-10 上：OneBot 带的识人术照样列出来（内置、照 im 的版本），总开关、设置都是 im 的配置项，没有程序、不收 plugins.call。
    let (home, _napcat, _) = started_installed(&Script::new([]), "", MEMBERS, |_| {}).await;
    let mut head = Head::connect(&home.root).await;
    let status = head.until("status", |status| status["web"].is_u64()).await;
    let web = u16::try_from(status["web"].as_u64().expect("有端口")).expect("是端口");
    let mut open = logged_in(&home.root, web).await;
    let id = "miyu-im-identity";

    let listed = call(&mut open, 1, "plugins.list", json!({})).await;
    let identity = one(&listed, id);
    assert_eq!(identity["builtin"], true, "{identity}");
    assert_eq!(identity["from"], "builtin", "{identity}");
    assert_eq!(identity["enabled"], true, "{identity}");
    assert_eq!(identity["settings"], "im.qq.identity_", "{identity}");
    assert_eq!(identity["platforms"], json!(["qq"]), "{identity}");

    let off = call(
        &mut open,
        2,
        "plugins.enable",
        json!({"package": id, "on": false}),
    )
    .await;
    assert_eq!(off["result"]["enabled"], false, "{off}");
    let listed = call(&mut open, 3, "plugins.list", json!({})).await;
    assert_eq!(one(&listed, id)["enabled"], false, "{listed}");
    call(
        &mut open,
        4,
        "plugins.enable",
        json!({"package": id, "on": true}),
    )
    .await;

    let got = call(&mut open, 5, "plugins.settings.get", json!({"package": id})).await;
    let keys: Vec<&str> = got["result"]["items"]
        .as_array()
        .expect("有项")
        .iter()
        .filter_map(|item| item["key"].as_str())
        .collect();
    assert_eq!(
        keys,
        [
            "im.qq.identity_admin",
            "im.qq.identity_card",
            "im.qq.identity_people"
        ],
        "总开关不列：{got}"
    );
    assert!(got["result"]["items"][0].get("hidden").is_none(), "{got}");
    let people = json!([{"key": "im.qq.identity_people", "value": ["主人=10001"]}]);
    let saved = call(
        &mut open,
        6,
        "plugins.settings.set",
        json!({"package": id, "changes": people}),
    )
    .await;
    assert!(saved["result"]["keys"].is_object(), "{saved}");
    let got = call(&mut open, 7, "plugins.settings.get", json!({"package": id})).await;
    assert_eq!(
        got["result"]["values"]["im.qq.identity_people"],
        json!(["主人=10001"])
    );
    let others = json!([{"key": "im.qq.listen", "value": 9000}]);
    let refused = call(
        &mut open,
        8,
        "plugins.settings.set",
        json!({"package": id, "changes": others}),
    )
    .await;
    assert_eq!(refused["error"]["reason"], "forbidden", "{refused}");
    let called = call(
        &mut open,
        9,
        "plugins.call",
        json!({"package": id, "method": "x"}),
    )
    .await;
    // 施工 Q-6 起驱动带的插件在进程里直接问：识人术没有这个方法。
    assert_eq!(called["error"]["reason"], "unknown_method", "{called}");
    stopped(home).await;
}
