//! 冒充提醒（施工 O-53，`onebot.md` 第一条「冒充提醒」）：真核心拉起真桥、假 NapCat。名字里含着终端管理员在这个群用过的名字、
//! 或者她自己的群名片的，不是那个人，就在他那一条后面记一块 `impostor`；同一个人借同一个名字只记一块；终端管理员的小号不算。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::group::*;
use crate::support::*;

/// 群号。
const GROUP: i64 = 670;

/// 群里的别人。
const LIN: i64 = 20002;
const WANG: i64 = 20003;

/// 终端管理员的小号（对应表里有，`support` 的 `CONFIG`）。
const ALT: i64 = 10007;

/// 假 NapCat 认得的群成员：她自己（群名片「米尤」）。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 群里不抽样：都只记下。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0 }\n";

/// `who` 用群名片 `card` 说第 `message` 条。
fn says(napcat: &Answering, who: i64, card: &str, message: i64) {
    napcat.send(group_frame(
        GROUP,
        who,
        message,
        json!([plain("大家好")]),
        (card, "x"),
    ));
}

/// 等到第 `message` 条判过：交回那时的群会话事件。
async fn until_decided(home: &Home, message: i64) -> Vec<Value> {
    let cause = format!("qq:{BOT}:{message}:{TIME}/decided");
    until_event(&home.root, &format!("qq:30003:group:{GROUP}"), |event| {
        event["cause"] == cause
    })
    .await
}

/// 记下的冒充提醒的字，照先后。
fn noted(events: &[Value]) -> Vec<String> {
    of_kind(events, "context.injected")
        .iter()
        .filter(|event| event["body"]["kind"] == "impostor")
        .filter_map(|event| {
            event["body"]["text"]
                .as_str()
                .map(|text| text.trim_end().to_string())
        })
        .collect()
}

/// 出厂的模板换出来的那一块。
fn fact(msg: i64, name: &str) -> String {
    std::fs::read_to_string(resources().join("packages/miyu-im/data/facts/impostor.txt"))
        .expect("读得出")
        .replace("{msg}", &msg.to_string())
        .replace("{name}", name)
        .trim_end()
        .to_string()
}

#[tokio::test]
async fn borrowing_the_admins_or_her_name_is_noted_once() {
    let (home, napcat, _) = started(&Script::new([]), RULES, "", MEMBERS).await;
    says(&napcat, ADMIN, "老板", 1);
    until_decided(&home, 1).await;
    // 小林叫「老板本人」：借了终端管理员的名字。
    says(&napcat, LIN, "老板本人", 2);
    let events = until_decided(&home, 2).await;
    assert_eq!(noted(&events), [fact(2, "老板")]);
    // 同一个人借同一个名字：不再记。
    says(&napcat, LIN, "老板本人", 3);
    until_decided(&home, 3).await;
    // 老王叫「米尤」：借了她的名字。终端管理员的小号叫「老板」：是他本人，不算。
    says(&napcat, WANG, "米尤 ", 4);
    says(&napcat, ALT, "老板", 5);
    let events = until_decided(&home, 5).await;
    assert_eq!(noted(&events), [fact(2, "老板"), fact(4, "米尤")]);
    // 只护着终端管理员和她的名字：借别人的名字不算。
    says(&napcat, 20008, "阿杰", 6);
    says(&napcat, 20009, "阿杰的朋友", 7);
    let events = until_decided(&home, 7).await;
    assert_eq!(noted(&events).len(), 2, "{:?}", noted(&events));
    // 判断照旧：只记下，不开轮。
    assert!(of_kind(&events, "turn.started").is_empty());
    // 在这个群关掉识人术（施工 I-10 上：场所开关照软件包插件的算，「陌生群」卡上的开关）：再有人借也不记。
    let mut head = crate::support::head::Head::connect(&home.root).await;
    let off = json!({"card": {"kind": "other_groups"}, "values": {"plugins": {"miyu-im-identity": false}}});
    let saved = head.page_call_with("cards.save_one", off).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    says(&napcat, 20011, "老板二号", 8);
    let events = until_decided(&home, 8).await;
    assert_eq!(noted(&events).len(), 2, "关了不记：{:?}", noted(&events));
    stopped(home).await;
}

/// 照网页、命令行的样子改系统配置。
async fn set(home: &Home, changes: Value) {
    let mut core = miyu_client::open::Core::connect_running(&home.root, "test")
        .await
        .expect("连得上核心");
    core.call(
        "i10",
        "config.set",
        json!({"layer": "system", "changes": changes}),
    )
    .await
    .unwrap_or_else(|refused| panic!("config.set 被拒：{refused}"));
}

#[tokio::test]
async fn the_identity_plugin_follows_its_settings_and_the_people_map() {
    // 施工 I-10 上：认借没借挪进 OneBot 带的识人术插件，照它的设置：识人映射、保护群名片关了、总开关关了。
    let settings = "qq.identity_people = [\"阿杰=20008\"]\nqq.identity_card = false\n";
    let (home, napcat, _) = started(&Script::new([]), RULES, settings, MEMBERS).await;
    // 映射里阿杰是 20008：本人不算，别人叫「阿杰的朋友」算。
    says(&napcat, 20008, "阿杰", 1);
    says(&napcat, 20009, "阿杰的朋友", 2);
    let events = until_decided(&home, 2).await;
    assert_eq!(noted(&events), [fact(2, "阿杰")]);
    // 保护群名片关了：叫「米尤」不算。
    says(&napcat, WANG, "米尤", 3);
    let events = until_decided(&home, 3).await;
    assert_eq!(noted(&events).len(), 1, "{:?}", noted(&events));
    // 总开关关了：借了映射里的名字也不记。
    set(&home, json!([{"key": "im.qq.identity_on", "value": false}])).await;
    // 配置推到桥要一会儿：每次换一个人借「阿杰」（同一个人只记一次），直到有一条没记，就是关上了；之后的一条也不该记。
    let mut message = 4;
    loop {
        let before = noted(&until_decided(&home, message - 1).await).len();
        says(&napcat, 30000 + message, "我是阿杰", message);
        let after = noted(&until_decided(&home, message).await).len();
        if after == before {
            break;
        }
        assert!(message < 40, "总开关一直没关上");
        message += 1;
    }
    says(&napcat, 31000, "阿杰本尊", 99);
    let events = until_decided(&home, 99).await;
    assert!(
        noted(&events).iter().all(|text| !text.contains("99")),
        "{:?}",
        noted(&events)
    );
    stopped(home).await;
}
