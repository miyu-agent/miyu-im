//! 后台页的方法：合计、明细的筛法、排行榜、记一笔、改（补汇率、换币种）、删。

use std::sync::{Arc, Mutex};

use serde_json::json;

use super::*;
use crate::rates::Rates;
use crate::settings::Settings;
use crate::wire::Wire;

/// 假的汇率：只认 1 USD = 7 CNY。
struct Fixed;

impl Rates for Fixed {
    fn rate(&self, from: &str, to: &str) -> Option<f64> {
        (from == "USD" && to == "CNY").then_some(7.0)
    }
}

/// 2026-10-20 12:00 东八区。
const NOW: i64 = 1_792_468_800_000;

fn state(name: &str) -> (std::path::PathBuf, State) {
    let dir =
        std::env::temp_dir().join(format!("im-sponsor-methods-{}-{name}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).expect("建得了");
    let book = Book::open(&dir.join("sponsors.json")).expect("开得了");
    let state = State {
        wire: Arc::new(Wire::new(Box::new(std::io::sink()))),
        book: Mutex::new(book),
        settings: Mutex::new(Settings::default()),
        rates: Box::new(Fixed),
    };
    (dir, state)
}

fn call_one(state: &State, method: &str, params: Value) -> Answer {
    let zone = TimeZone::fixed(jiff::tz::offset(8));
    call(
        state,
        &json!({"method": method, "params": params}),
        (NOW, &zone),
    )
}

#[test]
fn the_page_adds_lists_ranks_changes_and_deletes() {
    let (dir, state) = state("all");
    let added = call_one(
        &state,
        "add",
        json!({"platform": "qq", "user": "40001", "name": "小明", "amount": 50, "note": "服务器"}),
    )
    .expect("记得上");
    assert_eq!(added["entry"]["user"], "qq:40001", "号写成平台身份");
    call_one(
        &state,
        "add",
        json!({"platform": "qq", "user": "qq:40001", "amount": 10, "currency": "USD"}),
    )
    .expect("记得上");
    call_one(
        &state,
        "add",
        json!({"platform": "tg", "name": "Alice", "amount": 3, "currency": "EUR"}),
    )
    .expect("记得上");
    assert_eq!(
        call_one(&state, "add", json!({"platform": "qq", "amount": 1})).expect_err("没号没名字")["message"],
        "user or name is needed"
    );

    let totals = call_one(&state, "totals", json!({})).expect("有");
    assert_eq!(
        totals,
        json!({"base": "CNY", "month": 120.0, "year": 120.0, "all": 120.0, "pending": 1})
    );

    let ids = |answer: &Value| -> Vec<u64> {
        answer["entries"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry["id"].as_u64())
            .collect()
    };
    assert_eq!(
        ids(&call_one(&state, "list", json!({})).expect("有")),
        [3, 2, 1],
        "新的在前"
    );
    assert_eq!(
        ids(&call_one(&state, "list", json!({"query": "小明"})).expect("有")),
        [1]
    );
    assert_eq!(
        ids(&call_one(&state, "list", json!({"query": "40001"})).expect("有")),
        [2, 1],
        "搜号"
    );
    assert_eq!(
        ids(&call_one(&state, "list", json!({"platform": "tg"})).expect("有")),
        [3]
    );
    assert_eq!(
        ids(&call_one(&state, "list", json!({"from": NOW + 1})).expect("有")),
        Vec::<u64>::new()
    );
    assert_eq!(
        call_one(&state, "list", json!({"from": "昨天"})).expect_err("写错")["message"],
        "from must be milliseconds"
    );

    let ranking = call_one(&state, "ranking", json!({})).expect("有");
    assert_eq!(
        ranking["rows"],
        json!([{"user": "qq:40001", "name": "小明", "platform": "qq", "sum": 120.0, "count": 2}])
    );

    // 补汇率：欧元那一笔照给的汇率换算。
    let rated = call_one(&state, "update", json!({"id": 3, "rate": 7.9})).expect("改得了");
    assert_eq!(
        (
            rated["entry"]["rate"].as_f64(),
            rated["entry"]["converted"].as_f64()
        ),
        (Some(7.9), Some(23.7))
    );
    // 改金额：照原来的汇率重算。
    let more = call_one(&state, "update", json!({"id": 2, "amount": 20})).expect("改得了");
    assert_eq!(more["entry"]["converted"], 140.0);
    // 换成美元：照现在查的汇率。
    let dollars = call_one(&state, "update", json!({"id": 1, "currency": "USD"})).expect("改得了");
    assert_eq!(
        (
            dollars["entry"]["rate"].as_f64(),
            dollars["entry"]["converted"].as_f64()
        ),
        (Some(7.0), Some(350.0))
    );
    assert_eq!(
        call_one(&state, "update", json!({"id": 9, "amount": 1})).expect_err("没有")["message"],
        "not_found"
    );

    assert_eq!(
        call_one(&state, "delete", json!({"id": 3})).expect("删得了"),
        json!({"deleted": true})
    );
    assert_eq!(
        call_one(&state, "totals", json!({})).expect("有")["pending"],
        0
    );
    assert_eq!(
        call_one(&state, "rate", json!({"currency": "usd"})).expect("有"),
        json!({"base": "CNY", "rate": 7.0})
    );
    assert_eq!(
        call_one(&state, "rate", json!({"currency": "EUR"})).expect("有"),
        json!({"base": "CNY", "rate": null})
    );
    std::fs::remove_dir_all(dir).expect("删得掉");
}

#[test]
fn totals_follow_the_local_month_and_year() {
    let (dir, state) = state("periods");
    for (at, amount) in [
        (NOW, 10.0),
        (NOW - 40 * 86_400_000, 20.0),
        (NOW - 400 * 86_400_000, 30.0),
    ] {
        let new = New {
            platform: "qq".to_string(),
            user: Some("qq:1".to_string()),
            name: None,
            amount,
            currency: "CNY".to_string(),
            base: "CNY".to_string(),
            converted: Some(amount),
            rate: Some(1.0),
            at,
            note: None,
            by: None,
            venue: None,
        };
        state.book.lock().expect("锁得上").add(new).expect("记得上");
    }
    let totals = call_one(&state, "totals", json!({})).expect("有");
    assert_eq!(
        (
            totals["month"].as_f64(),
            totals["year"].as_f64(),
            totals["all"].as_f64()
        ),
        (Some(10.0), Some(30.0), Some(60.0))
    );
    std::fs::remove_dir_all(dir).expect("删得掉");
}

/// 照迁移程序转出来的样子写一笔旧版的记录（施工 I-131）：带原来的时刻和汇率。
fn old_one(user: Option<&str>, name: &str, at: i64) -> Value {
    json!({
        "platform": "qq", "user": user, "name": name, "amount": 10, "currency": "CNY",
        "base": "CNY", "converted": 10, "rate": 1, "at": at, "note": "QQ群红包赞助", "by": "qq:2433516202",
    })
}

#[test]
fn importing_keeps_the_original_times_and_importing_again_adds_nothing() {
    let (dir, state) = state("import");
    let entries = json!([
        old_one(Some("qq:40001"), "小明", 1_000),
        old_one(None, "只有名字", 2_000)
    ]);
    let first = call_one(&state, "import", json!({"entries": entries})).expect("导得进");
    assert_eq!(first, json!({"added": 2, "skipped": 0}));
    let listed = call_one(&state, "list", json!({})).expect("列得出");
    let ats: Vec<&Value> = listed["entries"]
        .as_array()
        .expect("有明细")
        .iter()
        .map(|entry| &entry["at"])
        .collect();
    assert_eq!(
        ats,
        [&json!(2_000), &json!(1_000)],
        "照原来的时刻，不换成此刻"
    );
    let again = call_one(&state, "import", json!({"entries": entries})).expect("导得进");
    assert_eq!(again, json!({"added": 0, "skipped": 2}), "重导是空操作");
    // 名字改了的、换了顺序的照样认得：有号认号。
    let renamed = json!([old_one(Some("qq:40001"), "改了名字", 1_000)]);
    let third = call_one(&state, "import", json!({"entries": renamed})).expect("导得进");
    assert_eq!(third, json!({"added": 0, "skipped": 1}));
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}

#[test]
fn what_is_exported_imports_back_into_an_empty_book() {
    let (dir, source) = state("export");
    call_one(
        &source,
        "add",
        json!({"platform": "qq", "user": "40001", "amount": 5, "currency": "USD"}),
    )
    .expect("记得上");
    let exported = call_one(&source, "export", json!({})).expect("导得出");
    assert_eq!(exported["version"], 1);
    let (other_dir, other) = state("export-target");
    let imported =
        call_one(&other, "import", json!({"entries": exported["entries"]})).expect("导得进");
    assert_eq!(imported, json!({"added": 1, "skipped": 0}));
    let back = call_one(&other, "export", json!({})).expect("导得出");
    assert_eq!(back, exported, "搬过去一模一样（编号、汇率、时刻都在）");
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
    std::fs::remove_dir_all(other_dir).expect("删得掉临时目录");
}

#[test]
fn one_wrong_entry_imports_nothing() {
    let (dir, state) = state("wrong");
    let mut bad = old_one(Some("qq:40001"), "小明", 0);
    bad["at"] = json!(0);
    let entries = json!([old_one(Some("qq:40002"), "对的", 1_000), bad]);
    let refused = call_one(&state, "import", json!({"entries": entries})).expect_err("不收");
    assert_eq!(refused["code"], -32602);
    assert_eq!(
        refused["message"],
        "entries[1]: at must be milliseconds since 1970"
    );
    let listed = call_one(&state, "list", json!({})).expect("列得出");
    assert_eq!(listed["entries"], json!([]), "对的那一笔也不导");
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}
