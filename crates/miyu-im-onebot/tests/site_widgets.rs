//! 插件往 im 页面加的内容（施工 I-8 下）：真核心照开关拉起真桥和两个测试插件，页面经通道要首页卡片、会话浮窗那一页、整页的内容：
//! 宿主调插件登记的方法（场所换成场所编号），照核心的界面块查过、照语言挑好字；写错的拒、记一行日志；没声明的不调。另有「放在首页」。

use serde_json::{Value, json};

use miyu_session::testkit::Script;
use miyu_store::root::DataRoot;

use crate::support::browser::{call, logged_in};
use crate::support::group::*;
use crate::support::head::Head;
use crate::support::hooked::install;
use crate::support::*;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// `dice` 带给 im 的声明：首页卡片、浮窗一页、一个整页。
const DECLARED: &str = "[plugin]\nname = { en = \"Dice\" }\n[plugin.ui]\nhome = true\npanel = true\n[[plugin.ui.pages]]\nid = \"stats\"\ntitle = { en = \"Stats\" }\nmethod = \"dice.stats\"\n";

/// 装两个插件：`dice` 照声明答三处（记下调用）；`sloppy` 只声明了一个整页，交的内容写错了。
fn plugins(root: &DataRoot) {
    let record = format!("record:{}", root.path().join("dice.calls").display());
    let answers = [
        record,
        r#"answer:chat.home:[{"type":"pairs","items":[{"label":"Rolled","value":"58"}]}]"#.to_string(),
        r#"answer:chat.panel:[{"type":"form","id":"f","fields":[{"key":"on","label":"On","control":"toggle","value":true}],"submit":{"label":"Save","call":"dice.save"}}]"#.to_string(),
        r#"answer:dice.stats:[{"type":"heading","text":{"en":"Stats","zh":"统计"}},{"type":"buttons","items":[{"label":"Again","action":{"open":"stats"}}]}]"#.to_string(),
    ];
    install(root, "dice", &answers, &["chat.inbound"]);
    std::fs::write(
        root.path().join("home/admin/packages/dice/im/plugin.toml"),
        DECLARED,
    )
    .expect("写得进");
    let sloppy = r#"answer:sloppy.page:[{"type":"nope"}]"#.to_string();
    install(root, "sloppy", &[sloppy], &["chat.inbound"]);
    let declared = "[plugin]\nname = { en = \"Sloppy\" }\n[[plugin.ui.pages]]\nid = \"p\"\ntitle = { en = \"P\" }\nmethod = \"sloppy.page\"\n";
    std::fs::write(
        root.path()
            .join("home/admin/packages/sloppy/im/plugin.toml"),
        declared,
    )
    .expect("写得进");
}

/// 要一块内容。
fn asking(package: &str, place: &str, more: Value) -> Value {
    let mut params = json!({"package": package, "place": place, "language": "zh"});
    params
        .as_object_mut()
        .expect("是对象")
        .extend(more.as_object().cloned().unwrap_or_default());
    params
}

#[tokio::test(flavor = "multi_thread")]
async fn plugin_content_is_fetched_checked_and_localized_for_the_page() {
    let (home, _napcat, _) = started_installed(&Script::new([]), "", MEMBERS, plugins).await;
    let mut head = Head::connect(&home.root).await;
    let status = head.until("status", |status| status["web"].is_u64()).await;
    let web = u16::try_from(status["web"].as_u64().expect("有端口")).expect("是端口");
    let mut open = logged_in(&home.root, web).await;

    let card = call(
        &mut open,
        1,
        "plugins.widget",
        asking("dice", "home", json!({})),
    )
    .await;
    assert_eq!(
        card["result"]["content"][0]["items"][0]["value"], "58",
        "{card}"
    );
    let venue = json!({"venue": {"type": "group", "id": "777"}});
    let panel = call(
        &mut open,
        2,
        "plugins.widget",
        asking("dice", "panel", venue),
    )
    .await;
    assert_eq!(panel["result"]["content"][0]["type"], "form", "{panel}");
    let calls = std::fs::read_to_string(home.root.path().join("dice.calls")).unwrap_or_default();
    let asked: Vec<Value> = calls
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let panel_call = asked
        .iter()
        .find(|one| one["method"] == "chat.panel")
        .expect("问了浮窗那一页");
    assert_eq!(
        panel_call["params"],
        json!({"platform": "qq", "venue": "qq:group:777"})
    );
    let page = call(
        &mut open,
        3,
        "plugins.widget",
        asking("dice", "page", json!({"page": "stats"})),
    )
    .await;
    assert_eq!(
        page["result"]["content"][0]["text"], "统计",
        "照语言挑：{page}"
    );

    // 写错的拒、记一行；没声明的、写错的参数不调。
    let sloppy = call(
        &mut open,
        4,
        "plugins.widget",
        asking("sloppy", "page", json!({"page": "p"})),
    )
    .await;
    assert_eq!(sloppy["error"]["reason"], "bad_widget_content", "{sloppy}");
    within("记一行", async {
        while !run_log(&home.root).contains("plugin widget rejected") {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    let wrongs = [
        asking("sloppy", "home", json!({})),
        asking(
            "sloppy",
            "panel",
            json!({"venue": {"type": "group", "id": "777"}}),
        ),
        asking("dice", "page", json!({"page": "nope"})),
        asking(
            "dice",
            "panel",
            json!({"venue": {"type": "group", "id": "x"}}),
        ),
        asking("dice", "status", json!({})),
        asking("im", "home", json!({})),
    ];
    for wrong in wrongs {
        let refused = call(&mut open, 5, "plugins.widget", wrong.clone()).await;
        assert_eq!(
            refused["error"]["reason"], "bad_params",
            "{wrong} {refused}"
        );
    }

    // 放在首页：出厂放，关了 plugins.list 跟着说。
    let listed = call(&mut open, 6, "plugins.list", json!({})).await;
    let dice = |listed: &Value| {
        listed["result"]["plugins"]
            .as_array()
            .and_then(|all| all.iter().find(|one| one["package"] == "dice").cloned())
            .expect("列出了 dice")
    };
    assert_eq!(dice(&listed)["on_home"], true, "{listed}");
    let off = call(
        &mut open,
        7,
        "plugins.home",
        json!({"package": "dice", "shown": false}),
    )
    .await;
    assert_eq!(off["result"], json!({"saved": true}), "{off}");
    let listed = call(&mut open, 8, "plugins.list", json!({})).await;
    assert_eq!(dice(&listed)["on_home"], false, "{listed}");
    let on = call(
        &mut open,
        10,
        "plugins.home",
        json!({"package": "dice", "shown": true}),
    )
    .await;
    assert_eq!(on["result"], json!({"saved": true}), "{on}");
    let listed = call(&mut open, 11, "plugins.list", json!({})).await;
    assert_eq!(dice(&listed)["on_home"], true, "放回来了：{listed}");
    let wrong = call(&mut open, 9, "plugins.home", json!({"package": "dice"})).await;
    assert_eq!(wrong["error"]["reason"], "bad_params", "{wrong}");
    stopped(home).await;
}
