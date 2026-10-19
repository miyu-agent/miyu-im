//! 卡片的对话模型（施工 O-58 补三）：真核心照开关拉起真桥，系统配置多一家 `judge`（判官的假服务器，`models.chat` 是 `judge/m`）。
//! 卡片设了模型的群，会话造好、交第一条消息以前就钉上（核心记 `session.policy_changed`）；卡片换了的跟着换，清掉的回到跟着配置
//! （`model: null`，核心记空的引用），没变的不再钉。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::events::venue_session;
use crate::support::group::*;
use crate::support::head::Head;
use crate::support::judge::judge;
use crate::support::*;

/// 群号。
const GROUP: i64 = 3921;

/// 卡片不设模型的群：人在别处给它的会话换的模型照旧。
const MANUAL: i64 = 3922;

/// 群里的人。
const LIN: i64 = 20004;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 系统的场所规则：群都不抽样。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0 }\n";

/// 会话日志里换模型的几条：照先后的模型（回到跟着配置的，核心记空的引用）。
fn pinned(events: &[Value]) -> Vec<Value> {
    events
        .iter()
        .filter(|event| {
            event["kind"] == "session.policy_changed" && event["body"].get("model").is_some()
        })
        .map(|event| event["body"]["model"].clone())
        .collect()
}

/// 存卡片：这个群设 `set`，[`MANUAL`] 不设。
async fn save(head: &mut Head, set: Value) {
    let cards = json!({"groups": [{"id": GROUP, "set": set}, {"id": MANUAL}]});
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
}

#[tokio::test]
async fn a_card_pins_the_model_and_follows_its_changes() {
    let server = judge(vec![]).await;
    let (home, napcat, _) =
        started_judged(&Script::new([]), &server, (RULES, ""), "", MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    save(&mut head, json!({"model": "judge/big"})).await;
    napcat.send(group_frame(
        GROUP,
        LIN,
        1,
        json!([plain("大家好")]),
        ("小林", "lin"),
    ));
    let venue = format!("qq:group:{GROUP}");
    let events = until_events(&home.root, &venue, |events| {
        events.iter().any(|event| event["kind"] == "message.user")
    })
    .await;
    // 另一个群：卡片没设，人在终端里给它的会话换了模型。
    napcat.send(group_frame(
        MANUAL,
        LIN,
        2,
        json!([plain("在吗")]),
        ("小林", "lin"),
    ));
    let manual = format!("qq:group:{MANUAL}");
    until_events(&home.root, &manual, |events| !events.is_empty()).await;
    let (session, _) = venue_session(&home.root, &manual).expect("有会话");
    let set = head
        .call(
            "session.configure",
            json!({"session": session, "model": "judge/manual"}),
        )
        .await;
    assert!(set.get("result").is_some(), "{set}");
    let kinds: Vec<&str> = events
        .iter()
        .filter_map(|event| event["kind"].as_str())
        .collect();
    assert_eq!(
        kinds[..3],
        ["session.created", "session.policy_changed", "message.user"],
        "交第一条消息以前钉上"
    );
    assert_eq!(pinned(&events), [json!("judge/big")]);
    // 别的格变了、模型没变：不再钉。再换模型：照新的钉。
    save(&mut head, json!({"model": "judge/big", "show_ids": true})).await;
    save(&mut head, json!({"model": "judge/small"})).await;
    let events = until_events(&home.root, &venue, |events| pinned(events).len() >= 2).await;
    assert_eq!(pinned(&events), [json!("judge/big"), json!("judge/small")]);
    // 清掉：回到跟着配置（桥交 `model: null`，核心的日志记空的引用）。
    save(&mut head, json!({})).await;
    let events = until_events(&home.root, &venue, |events| pinned(events).len() >= 3).await;
    assert_eq!(
        pinned(&events),
        [json!("judge/big"), json!("judge/small"), json!("")]
    );
    // 两页改一张卡（`cards.save_one`）也跟着换。
    let one = json!({"card": {"kind": "group", "id": GROUP}, "values": {"model": "judge/page"}});
    let saved = head.page_call_with("cards.save_one", one).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let events = until_events(&home.root, &venue, |events| pinned(events).len() >= 4).await;
    assert_eq!(pinned(&events)[3], json!("judge/page"));
    // 卡片没设的群，存了几次卡也不碰：照旧是人换的那一个。
    let (_, events) = venue_session(&home.root, &manual).expect("有会话");
    assert_eq!(pinned(&events), [json!("judge/manual")]);
    stopped(home).await;
}

/// 会话日志里换人格的几条：照先后的人格。
fn personas(events: &[Value]) -> Vec<Value> {
    events
        .iter()
        .filter(|event| {
            event["kind"] == "session.policy_changed" && event["body"].get("persona").is_some()
        })
        .map(|event| event["body"]["persona"].clone())
        .collect()
}

#[tokio::test]
async fn a_card_persona_switches_the_existing_session() {
    // 施工 O-58 补四：已经造好的场所会话，卡片换了人格就换（核心先压缩，下一轮换上）；清掉的不发（核心的 null 是无人格）。
    let server = judge(vec![]).await;
    let (home, napcat, _) =
        started_judged(&Script::new([]), &server, (RULES, ""), "", MEMBERS).await;
    miyu_session::testkit::install_sample_persona(&home.root);
    let other = home.root.system().join("personas").join("other");
    std::fs::create_dir_all(other.join("prompts")).expect("建得了目录");
    let sample = miyu_session::testkit::sample_persona_dir();
    for file in ["persona.toml", "prompts/persona.md"] {
        std::fs::copy(sample.join(file), other.join(file)).expect("复制得了");
    }
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let first = miyu_session::testkit::SAMPLE_PERSONA;
    save(&mut head, json!({"persona": first})).await;
    napcat.send(group_frame(
        GROUP,
        LIN,
        1,
        json!([plain("大家好")]),
        ("小林", "lin"),
    ));
    let venue = format!("qq:group:{GROUP}");
    let events = until_events(&home.root, &venue, |events| {
        events.iter().any(|event| event["kind"] == "message.user")
    })
    .await;
    assert_eq!(
        events[0]["body"]["persona"], first,
        "造会话时带上：{}",
        events[0]
    );
    assert!(personas(&events).is_empty(), "一样的不再换：{events:?}");
    save(&mut head, json!({"persona": "other"})).await;
    let events = until_events(&home.root, &venue, |events| !personas(events).is_empty()).await;
    assert_eq!(personas(&events), [json!("other")]);
    // 清掉人格、换模型：只换模型（等到它，人格那一条照旧只有一条）。
    save(&mut head, json!({"model": "judge/big"})).await;
    let events = until_events(&home.root, &venue, |events| !pinned(events).is_empty()).await;
    assert_eq!(pinned(&events), [json!("judge/big")]);
    assert_eq!(personas(&events), [json!("other")], "清掉的人格不发");
    stopped(home).await;
}
