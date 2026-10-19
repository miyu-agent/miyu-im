//! im 页面上的会话页（施工 I-7 上）：真核心拉起真桥、假 NapCat，页面登录以后经通道调 `cards.get`、`cards.add`、`cards.save_one`、
//! `cards.remove`，页面服务在这份登录的人的连接上经核心的 `package.call` 转给桥自己；卡片、白名单变了通道推 `cards`。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::browser::{Channel, call_noting, event, logged_in};
use crate::support::group::*;
use crate::support::head::Head;
use crate::support::*;

/// 群里的别人、要加进白名单的人。
const LIN: i64 = 20004;

/// 配置里本来就在白名单、还没有卡的人。
const ZHE: i64 = 20007;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 在通道上调 `method`：交回 `result`，并等到推来 `cards`（赶在回应前头的也算）。
async fn changing(open: &mut Channel, id: u64, method: &str, params: Value) -> Value {
    let (answer, pushed) = call_noting(open, id, method, params).await;
    if !pushed.contains(&json!({"event": "cards"})) {
        within("推来 cards", event(open, "cards")).await;
    }
    answer["result"].clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_venue_page_reads_and_changes_the_cards_through_the_core() {
    let whitelist = format!("whitelist = [\"qq:{ZHE}\"]\n");
    let (home, _napcat, _) = started(&Script::new([]), "", &whitelist, MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    let status = head
        .until("status", |status| {
            status["napcat"]["connected"] == true && status["web"].is_u64()
        })
        .await;
    let web = u16::try_from(status["web"].as_u64().expect("有端口")).expect("是端口");
    let mut open = logged_in(&home.root, web).await;

    // 读：卡片、默认值、AI 在的群和好友（带问到它的机器人）。
    let (got, _) = call_noting(&mut open, 1, "cards.get", json!({})).await;
    let got = &got["result"];
    assert_eq!(got["cards"]["groups"], json!([]), "{got}");
    assert_eq!(got["groups"][0]["bot"], BOT, "{got}");
    assert!(got["defaults"]["other_groups"].is_object(), "{got}");
    // 白名单成员还没有卡的也有默认值（会话页上也是一张卡）。
    assert_eq!(got["whitelist"], json!([ZHE]), "{got}");
    assert_eq!(got["cards"]["privates"], json!([]), "{got}");
    assert!(
        got["defaults"]["privates"][ZHE.to_string()].is_object(),
        "{got}"
    );

    // 加一个群：答号、推 cards；默认回复。
    let added = changing(&mut open, 2, "cards.add", json!({"value": "777"})).await;
    assert_eq!(added, json!({"id": "777"}));
    let (got, _) = call_noting(&mut open, 3, "cards.get", json!({})).await;
    assert_eq!(
        got["result"]["cards"]["groups"],
        json!([{"id": 777, "set": {}}])
    );
    assert_eq!(
        got["result"]["defaults"]["groups"]["777"]["on"]["value"],
        true
    );

    // 选模型的下拉（施工 I-7 中一）：照核心的 `model.list`，只交引用、池、配的对话和看图模型。
    let (models, _) = call_noting(&mut open, 19, "models", json!({})).await;
    let models = &models["result"];
    assert!(models["models"].is_array(), "{models}");
    assert!(models["pools"].is_array(), "{models}");
    assert!(models["uses"].is_object(), "{models}");
    assert!(
        models["models"]
            .as_array()
            .into_iter()
            .flatten()
            .all(|one| one["ref"].is_string()),
        "{models}"
    );

    // 能贴的表情（施工 I-7 中三下二）：驱动给的列表，出厂贴的那一个。
    let (reactions, _) = call_noting(&mut open, 18, "reactions", json!({})).await;
    let reactions = &reactions["result"];
    assert_eq!(reactions["default"], "289", "{reactions}");
    assert_eq!(
        reactions["reactions"][0],
        json!({"id": "289", "name": "睁眼"}),
        "{reactions}"
    );
    assert!(
        reactions["reactions"]
            .as_array()
            .is_some_and(|all| all.iter().any(|one| one["glyph"] == "👍")),
        "{reactions}"
    );

    // 卡上 `rules`（施工 I-7 中一）：照键存；写法不对的键报在 `rules.<键>` 上。
    let card = json!({"kind": "group", "id": 777});
    let values = json!({"rules": {"chatty.base": 0.7, "discipline": "when-called"}});
    let saved = changing(
        &mut open,
        20,
        "cards.save_one",
        json!({"card": card, "values": values}),
    )
    .await;
    assert_eq!(saved, json!({"saved": true}));
    let (got, _) = call_noting(&mut open, 21, "cards.get", json!({})).await;
    let set = &got["result"]["cards"]["groups"][0]["set"];
    assert_eq!(
        set["rules"],
        json!({"chatty.base": 0.7, "discipline": "when-called"}),
        "{got}"
    );
    let wrong = json!({"rules": {"allow": true}});
    let (refused, _) = call_noting(
        &mut open,
        22,
        "cards.save_one",
        json!({"card": card, "values": wrong}),
    )
    .await;
    assert_eq!(refused["result"]["saved"], false, "{refused}");
    assert_eq!(
        refused["result"]["problems"][0]["key"], "rules.allow",
        "{refused}"
    );
    let cleared = json!({"rules": {"chatty.base": null, "discipline": null}});
    let saved = changing(
        &mut open,
        23,
        "cards.save_one",
        json!({"card": card, "values": cleared}),
    )
    .await;
    assert_eq!(saved, json!({"saved": true}));

    // 关掉回复：只写这一格。
    let card = json!({"kind": "group", "id": 777});
    let saved = changing(
        &mut open,
        4,
        "cards.save_one",
        json!({"card": card, "values": {"on": false}}),
    )
    .await;
    assert_eq!(saved, json!({"saved": true}));
    let (got, _) = call_noting(&mut open, 5, "cards.get", json!({})).await;
    assert_eq!(
        got["result"]["cards"]["groups"][0]["set"],
        json!({"on": false})
    );

    // 加一个人：进白名单，还没有卡也有默认值。
    let added = changing(
        &mut open,
        6,
        "cards.add",
        json!({"value": format!("p:{LIN}")}),
    )
    .await;
    assert_eq!(added, json!({"id": LIN.to_string()}));
    let (got, _) = call_noting(&mut open, 7, "cards.get", json!({})).await;
    assert_eq!(got["result"]["whitelist"], json!([ZHE, LIN]), "{got}");
    assert!(
        got["result"]["defaults"]["privates"][LIN.to_string()].is_object(),
        "{got}"
    );

    // 移出：人出白名单，群出名单。
    let private = json!({"card": {"kind": "private", "id": LIN}});
    assert_eq!(
        changing(&mut open, 8, "cards.remove", private).await,
        json!({"saved": true})
    );
    assert_eq!(
        changing(&mut open, 9, "cards.remove", json!({"card": card})).await,
        json!({"saved": true})
    );
    let (got, _) = call_noting(&mut open, 10, "cards.get", json!({})).await;
    assert_eq!(got["result"]["whitelist"], json!([ZHE]), "{got}");
    assert_eq!(got["result"]["cards"]["groups"], json!([]), "{got}");

    // 写错的读的一头拒，核心转成 `method_failed`；整份存（cards.save）不转。
    let (bad, _) = call_noting(&mut open, 11, "cards.add", json!({"value": "x"})).await;
    assert_eq!(bad["error"]["reason"], "method_failed", "{bad}");
    let (whole, _) = call_noting(&mut open, 12, "cards.save", json!({"cards": {}})).await;
    assert_eq!(whole["error"]["reason"], "unknown_method", "{whole}");
}
