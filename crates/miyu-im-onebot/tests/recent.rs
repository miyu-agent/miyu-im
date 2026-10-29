//! 回复时带多少条群聊记录（施工 O-62）：群卡的 `recent` 拼成场所规则的 `recent`，桥造群会话时经 `venue.session` 交给核心（核心
//! 3161f0c5 起认，造会话时钉进快照）。真核心照开关拉起真桥：设了的群会话钉着这个数，没设的照出厂的 30 条（施工 I-16）。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::*;

/// 设了条数的群。
const COUNTED: i64 = 3911;

/// 没设的群。
const PLAIN: i64 = 3912;

/// 群里的人。
const LIN: i64 = 20004;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 系统的场所规则：群都不抽样。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0 }\n";

#[tokio::test]
async fn a_group_card_pins_its_recent_count_on_new_sessions() {
    let (home, napcat, _) = started(&Script::new([]), RULES, "", MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let cards = json!({"groups": [{"id": COUNTED, "set": {"recent": 50}}, {"id": PLAIN}]});
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let mut created = Vec::new();
    for (n, group) in [COUNTED, PLAIN].into_iter().enumerate() {
        let message = i64::try_from(n).expect("小") + 1;
        napcat.send(group_frame(
            group,
            LIN,
            message,
            json!([plain("大家好")]),
            ("小林", "lin"),
        ));
        let venue = format!("qq:group:{group}");
        let events = until_events(&home.root, &venue, |events| !events.is_empty()).await;
        created.push(events[0].clone());
    }
    assert_eq!(limit(&home, &created[0]), Some(json!(50)), "{}", created[0]);
    assert_eq!(
        limit(&home, &created[1]),
        Some(json!(30)),
        "没设的照出厂：{}",
        created[1]
    );
    stopped(home).await;
}

/// 会话造出来的那一条 `created` 钉下的快照（数据根的 blob，照 `policy` 的摘要找）里，群聊近况那一格的 `limit`。
fn limit(home: &Home, created: &Value) -> Option<Value> {
    let digest = created["body"]["policy"].as_str().expect("有快照");
    let hex = digest.strip_prefix("sha256:").expect("写法");
    let blobs = home.root.blobs();
    let file = find(&blobs, hex).unwrap_or_else(|| panic!("{} 里没有 {hex}", blobs.display()));
    let snapshot: Value =
        serde_json::from_slice(&std::fs::read(file).expect("读得了")).expect("快照是 JSON");
    recent(&snapshot)
        .unwrap_or_else(|| panic!("快照里没有群聊近况：{snapshot}"))
        .get("limit")
        .cloned()
}

/// `dir` 底下名字里有 `hex` 的文件。
fn find(dir: &std::path::Path, hex: &str) -> Option<std::path::PathBuf> {
    for entry in std::fs::read_dir(dir).ok()? {
        let path = entry.ok()?.path();
        if path.is_dir() {
            if let Some(found) = find(&path, hex) {
                return Some(found);
            }
        } else if path
            .to_string_lossy()
            .replace(['/', '\\'], "")
            .contains(hex)
        {
            return Some(path);
        }
    }
    None
}

/// 快照里群聊近况那一格：带 `budget` 的对象。
fn recent(value: &Value) -> Option<&Value> {
    match value {
        Value::Object(map) if map.contains_key("budget") && map.contains_key("omitted") => {
            Some(value)
        }
        Value::Object(map) => map.values().find_map(recent),
        Value::Array(items) => items.iter().find_map(recent),
        _ => None,
    }
}
