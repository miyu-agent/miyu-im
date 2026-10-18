//! 场所卡片的两页（施工 O-58 下，`onebot.md`「场所卡片的两页」）：真核心照开关拉起真桥、假 NapCat，头照终端、网页的样子订阅整页、
//! 经 `package.call` 调三个方法。「群」页打开就推内容（「其他群」在前、名单里的群照群名、可加的是她在的别的群）；存一张表单只改
//! 交来的格、写进盘、推新的；写错的答问题；加群、删群；「其他群」删不掉。「私聊」页加一个人把他写进系统配置的白名单、推出他那一项，
//! 删掉把他移出；白名单在别处改了也推出他那一项，他还没有卡的存一张表单就建上。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::*;

/// 白名单成员（加进来、删掉的那一个）。
const LIN: i64 = 20004;

/// 名单里的群。
const LISTED: i64 = 777;

/// 她在、还没有卡的群。
const OTHER: i64 = 666;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 订阅整页 `widget`，交回第一份有内容的。
async fn open(head: &mut Head, widget: &str) -> Value {
    let page = json!({"package": "im", "widget": widget});
    let reply = head
        .call(
            "subscribe",
            json!({"stream": "widgets", "showing": [], "page": page}),
        )
        .await;
    let listed = reply["result"]["widgets"].as_array().expect("几块").clone();
    if let Some(content) = listed
        .iter()
        .find(|one| one["package"] == "im" && one["id"] == widget)
        .map(|one| one["content"].clone())
        .filter(Value::is_array)
    {
        return content;
    }
    pushed(head, widget, |_| true).await
}

/// 等到 `widget` 推来一份合 `wanted` 的内容。
async fn pushed(head: &mut Head, widget: &str, wanted: impl Fn(&Value) -> bool) -> Value {
    head.pushed(|push| {
        let one = &push["params"]["widget"];
        push["method"] == "widget.changed"
            && push["params"]["id"] == widget
            && one["content"].is_array()
            && wanted(&one["content"])
    })
    .await["params"]["widget"]["content"]
        .clone()
}

/// 内容里那几项的编号。
fn ids(content: &Value) -> Vec<String> {
    content[1]["items"]
        .as_array()
        .expect("几项")
        .iter()
        .map(|item| item["id"].as_str().expect("编号").to_string())
        .collect()
}

/// 一项里键是 `key` 的那一格的值。
fn value_of(content: &Value, item: &str, key: &str) -> Value {
    content[1]["items"]
        .as_array()
        .expect("几项")
        .iter()
        .find(|one| one["id"] == item)
        .and_then(|one| {
            one["blocks"]
                .as_array()?
                .iter()
                .filter_map(|block| block["fields"].as_array())
                .flatten()
                .find(|field| field["key"] == key)
                .map(|field| field["value"].clone())
        })
        .expect("有这一格")
}

/// 盘上的 `cards.json`。
fn on_disk(home: &Home) -> Value {
    let path = home.root.state().join("packages/im/cards.json");
    serde_json::from_slice(&std::fs::read(path).expect("读得出")).expect("是 JSON")
}

/// 起来、等 NapCat 连上，交回头。
async fn up() -> (Home, Answering, Head) {
    let (home, napcat, _) = started(&Script::new([]), "", "", MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    (home, napcat, head)
}

#[tokio::test]
async fn the_groups_page_edits_adds_and_removes_group_cards() {
    let (home, _napcat, mut head) = up().await;
    let cards = json!({"cards": {"groups": [{"id": LISTED}]}});
    assert_eq!(
        head.page_call_with("cards.save", cards).await["result"],
        json!({"saved": true})
    );
    let content = open(&mut head, "groups").await;
    assert_eq!(ids(&content), ["other", "777"]);
    assert_eq!(content[1]["items"][1]["title"], "测试群");
    assert_eq!(
        content[1]["add"]["options"],
        json!([{"value": "666", "label": "另一个群（666）"}])
    );
    assert_eq!(content[0]["items"][0]["action"], json!({"open": "friends"}));
    // 存一张表单：只交改过的格。
    let card = json!({"kind": "group", "id": LISTED});
    let saved = head
        .page_call_with(
            "cards.save_one",
            json!({"card": card, "values": {"rate": "2/60s", "managers": ["1001"]}}),
        )
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    assert_eq!(
        on_disk(&home)["groups"][0]["set"],
        json!({"rate": "2/60s", "managers": ["qq:1001"]})
    );
    let content = pushed(&mut head, "groups", |content| {
        value_of(content, "777", "rate") == "2/60s"
    })
    .await;
    assert_eq!(
        value_of(&content, "777", "managers"),
        json!(["1001"]),
        "页面上只写号"
    );
    // 写错的：答问题，盘上不动。
    let refused = head
        .page_call_with(
            "cards.save_one",
            json!({"card": card, "values": {"rate": "lots", "on": "yes"}}),
        )
        .await;
    assert_eq!(refused["result"]["saved"], false, "{refused}");
    let keys: Vec<&Value> = refused["result"]["problems"]
        .as_array()
        .expect("几条")
        .iter()
        .map(|one| &one["key"])
        .collect();
    assert_eq!(
        keys,
        [&json!("on")],
        "类型不对的先报，规则层的等它改对了再查：{refused}"
    );
    let refused = head
        .page_call_with(
            "cards.save_one",
            json!({"card": card, "values": {"rate": "lots"}}),
        )
        .await;
    assert_eq!(
        refused["result"]["problems"],
        json!([{"key": "rate", "message": "要写成 3/60s 这样：60 秒最多 3 条"}]),
        "照那一格该怎么写说，不给人看原因码：{refused}"
    );
    assert_eq!(on_disk(&home)["groups"][0]["set"]["rate"], "2/60s");
    // 加群、删群；「其他群」删不掉。
    let added = head
        .page_call_with("cards.add", json!({"value": OTHER.to_string()}))
        .await;
    assert_eq!(added["result"], json!({"id": "666"}), "{added}");
    pushed(&mut head, "groups", |content| {
        ids(content).contains(&"666".to_string())
    })
    .await;
    let removed = head
        .page_call_with(
            "cards.remove",
            json!({"card": {"kind": "group", "id": OTHER}}),
        )
        .await;
    assert_eq!(removed["result"], json!({"saved": true}));
    let groups: Vec<Value> = on_disk(&home)["groups"]
        .as_array()
        .expect("几张")
        .iter()
        .map(|one| one["id"].clone())
        .collect();
    assert_eq!(groups, [json!(LISTED)]);
    let other = head
        .page_call_with("cards.remove", json!({"card": {"kind": "other_groups"}}))
        .await;
    assert!(other.get("error").is_some(), "其他群删不掉：{other}");
    stopped(home).await;
}

#[tokio::test]
async fn the_friends_page_adds_and_removes_whitelist_members() {
    let (home, _napcat, mut head) = up().await;
    let content = open(&mut head, "friends").await;
    assert_eq!(ids(&content), ["other"], "白名单是空的：只有「其他私聊」");
    let options = content[1]["add"]["options"]
        .as_array()
        .expect("可加的")
        .clone();
    assert!(
        options.contains(&json!({"value": "p:20004", "label": "lin（20004）"})),
        "{options:?}"
    );
    let added = head
        .page_call_with("cards.add", json!({"value": format!("p:{LIN}")}))
        .await;
    assert_eq!(added["result"], json!({"id": "20004"}), "{added}");
    let config =
        std::fs::read_to_string(home.root.path().join("system/config.toml")).expect("读得出");
    assert!(config.contains("qq:20004"), "写进了白名单：{config}");
    let content = pushed(&mut head, "friends", |content| {
        ids(content) == ["other", "20004"]
    })
    .await;
    assert_eq!(content[1]["items"][1]["title"], "lin");
    // 白名单成员还没有设过的格：存一张表单就建上他的卡。
    let card = json!({"kind": "private", "id": LIN});
    let saved = head
        .page_call_with(
            "cards.save_one",
            json!({"card": card, "values": {"long_text": "split"}}),
        )
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    assert_eq!(
        on_disk(&home)["privates"],
        json!([{"id": LIN, "set": {"long_text": "split"}}])
    );
    let removed = head
        .page_call_with("cards.remove", json!({"card": card}))
        .await;
    assert_eq!(removed["result"], json!({"saved": true}), "{removed}");
    let config =
        std::fs::read_to_string(home.root.path().join("system/config.toml")).expect("读得出");
    assert!(!config.contains("qq:20004"), "移出了白名单：{config}");
    assert_eq!(on_disk(&home)["privates"], json!([]));
    pushed(&mut head, "friends", |content| ids(content) == ["other"]).await;
    stopped(home).await;
}

#[tokio::test]
async fn a_whitelist_changed_elsewhere_reaches_the_friends_page() {
    // 白名单在别处改了（终端里改系统配置）：私聊页推出他那一项；他还没有卡，存一张表单就建上。
    let (home, _napcat, mut head) = up().await;
    assert_eq!(ids(&open(&mut head, "friends").await), ["other"]);
    let change = json!({"key": "im.whitelist", "value": [format!("qq:{LIN}")]});
    let set = head
        .call(
            "config.set",
            json!({"layer": "system", "changes": [change]}),
        )
        .await;
    assert!(set.get("result").is_some(), "{set}");
    pushed(&mut head, "friends", |content| {
        ids(content) == ["other", "20004"]
    })
    .await;
    let card = json!({"kind": "private", "id": LIN});
    let saved = head
        .page_call_with(
            "cards.save_one",
            json!({"card": card, "values": {"final_only": true}}),
        )
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    assert_eq!(
        on_disk(&home)["privates"],
        json!([{"id": LIN, "set": {"final_only": true}}])
    );
    stopped(home).await;
}
