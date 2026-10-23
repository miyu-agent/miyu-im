//! 工具 `sponsor`：参数、认人、只认管理员记账、换算、查一个人、排行榜。

use std::path::PathBuf;

use serde_json::json;

use super::*;

/// 假的汇率：只认 1 USD = 7.144 CNY。
struct Fixed;

impl Rates for Fixed {
    fn rate(&self, from: &str, to: &str) -> Option<f64> {
        (from == "USD" && to == "CNY").then_some(7.144)
    }
}

/// 一个临时的账本。
fn book(name: &str) -> (PathBuf, Mutex<Book>) {
    let dir = std::env::temp_dir().join(format!("im-sponsor-tool-{}-{name}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).expect("建得了");
    let book = Book::open(&dir.join("sponsors.json")).expect("开得了");
    (dir, Mutex::new(book))
}

fn context<'a>(book: &'a Mutex<Book>) -> Context<'a> {
    Context {
        book,
        settings: Settings::default(),
        rates: &Fixed,
        now: 1_792_468_800_000,
        zone: TimeZone::fixed(jiff::tz::offset(8)),
    }
}

/// 群里管理员（场所的管理的人）叫的一轮，参数 `args`。
fn by_manager(args: Value) -> Value {
    json!({"session": "s", "args": args, "by": {"kind": "external", "id": "qq:9", "role": "manager"}, "owner": false})
}

/// im 的 `chat.venue`：群 777，叫她的那一条引用了 `quote`、@ 了 `mentions`。
fn venue(quote: Option<(&str, &str)>, mentions: &[(&str, &str)]) -> Value {
    let person = |(id, name): (&str, &str)| json!({"id": id, "name": name});
    json!({"platform": "qq", "venue": "qq:group:777", "bot": 10000, "origin": {
        "sender": {"id": "qq:9", "name": "老板"},
        "quote": quote.map(|one| json!({"msg": "5", "sender": person(one)})),
        "mentions": mentions.iter().copied().map(person).collect::<Vec<_>>(),
    }})
}

fn text(result: &Value) -> &str {
    result["blocks"][0]["text"].as_str().unwrap_or_default()
}

#[test]
fn arguments_say_what_is_wrong() {
    assert_eq!(
        read(&json!({})),
        Err("action must be add, person or ranking")
    );
    assert_eq!(
        read(&json!({"action": "add", "amount": -1})),
        Err("amount must be a number above 0")
    );
    assert_eq!(
        read(&json!({"action": "add", "currency": "dollar"})),
        Err("currency must be a three-letter code such as CNY or USD")
    );
    assert_eq!(
        read(&json!({"action": "ranking", "period": "week"})),
        Err("period must be month, year or all")
    );
    let asked = read(&json!({"action": "add", "user": "40001", "amount": "50", "currency": "usd", "note": " 谢谢\n "})).expect("读得出");
    assert_eq!(
        (
            asked.user,
            asked.amount,
            asked.currency.as_deref(),
            asked.note.as_deref()
        ),
        (Some(40001), Some(50.0), Some("USD"), Some("谢谢"))
    );
}

#[test]
fn the_sponsor_is_the_number_the_quote_the_one_mention_or_a_nickname() {
    let asked = |args: Value| read(&args).expect("读得出");
    let who_of = |args: Value, venue: &Value| who(&asked(args), venue).map(|who| who.spoken());
    let quoted = venue(Some(("qq:40001", "小明")), &[("qq:40002", "小红")]);
    assert_eq!(
        who_of(json!({"action": "add", "user": 40002}), &quoted).as_deref(),
        Some("小红 (qq:40002)"),
        "号写了的照号，名字照认得的"
    );
    assert_eq!(
        who_of(json!({"action": "add"}), &quoted).as_deref(),
        Some("小明 (qq:40001)"),
        "没写号的照引用的人"
    );
    let mentioned = venue(None, &[("qq:40002", "小红")]);
    assert_eq!(
        who_of(json!({"action": "add"}), &mentioned).as_deref(),
        Some("小红 (qq:40002)"),
        "唯一 @ 的那一个"
    );
    let two = venue(None, &[("qq:40002", "小红"), ("qq:40003", "小刚")]);
    assert_eq!(
        who_of(json!({"action": "add"}), &two),
        None,
        "@ 了两个认不出"
    );
    assert_eq!(
        who_of(json!({"action": "add", "name": "路人"}), &two).as_deref(),
        Some("路人"),
        "只有昵称的"
    );
    let her = venue(Some(("qq:10000", "米尤")), &[]);
    assert_eq!(
        who_of(json!({"action": "add"}), &her),
        None,
        "引用她自己的不算"
    );
}

#[test]
fn only_an_admin_turn_records_and_other_currencies_are_converted_and_frozen() {
    let (dir, book) = book("add");
    let context = context(&book);
    let member = json!({"session": "s", "args": {"action": "add", "user": 1, "amount": 5}, "by": {"kind": "external", "id": "qq:3", "role": "member"}, "owner": false});
    let refused = call(&member, &venue(None, &[]), &context);
    assert_eq!(
        (text(&refused), refused["error"].as_bool()),
        (say("admin-only", &[]).as_str(), Some(true))
    );
    assert!(book.lock().expect("锁得上").entries().is_empty(), "没记上");

    let added = call(
        &by_manager(json!({"action": "add", "amount": 50})),
        &venue(Some(("qq:40001", "小明")), &[]),
        &context,
    );
    assert_eq!(text(&added), "Recorded #1: 小明 (qq:40001) gave 50 CNY.");
    let dollars = call(
        &by_manager(json!({"action": "add", "user": 40001, "amount": 10, "currency": "USD"})),
        &venue(None, &[]),
        &context,
    );
    assert_eq!(
        text(&dollars),
        "Recorded #2: qq:40001 gave 10 USD (= 71.44 CNY at 7.144)."
    );
    let euros = call(
        &by_manager(json!({"action": "add", "name": "路人", "amount": 3, "currency": "EUR"})),
        &venue(None, &[]),
        &context,
    );
    assert_eq!(
        text(&euros),
        "Recorded #3: 路人 gave 3 EUR (not converted to CNY yet)."
    );
    let owner = json!({"session": "s", "args": {"action": "add", "user": 2, "amount": 1}, "by": {"kind": "person", "account": "owner", "via": "qq:9"}, "owner": true});
    assert_eq!(
        call(&owner, &venue(None, &[]), &context)["error"],
        false,
        "主人私聊里记也行"
    );
    let entries = book.lock().expect("锁得上").entries().to_vec();
    assert_eq!(
        (
            entries[1].rate,
            entries[1].converted,
            entries[1].by.as_deref()
        ),
        (Some(7.144), Some(71.44), Some("qq:9"))
    );
    assert_eq!(
        (entries[2].user.as_deref(), entries[2].converted),
        (None, None)
    );
    assert_eq!(
        entries[3].by.as_deref(),
        Some("qq:9"),
        "私聊里认出的本人照 via"
    );
    std::fs::remove_dir_all(dir).expect("删得掉");
}

#[test]
fn anyone_looks_up_a_sponsor_and_the_ranking() {
    let (dir, book) = book("look");
    let context = context(&book);
    for args in [
        json!({"action": "add", "user": 40001, "name": "小明", "amount": 50, "note": "服务器费"}),
        json!({"action": "add", "user": 40001, "amount": 10, "currency": "USD"}),
        json!({"action": "add", "name": "路人", "amount": 80}),
        json!({"action": "add", "user": 40001, "amount": 3, "currency": "EUR"}),
    ] {
        assert_eq!(
            call(&by_manager(args), &venue(None, &[]), &context)["error"],
            false
        );
    }
    let member = |args: Value| json!({"session": "s", "args": args, "by": {"kind": "external", "id": "qq:3", "role": "member"}, "owner": false});
    let one = call(
        &member(json!({"action": "person", "user": 40001})),
        &venue(None, &[]),
        &context,
    );
    assert_eq!(
        text(&one),
        "小明 (qq:40001): 3 records, total 121.44 CNY, 1 not converted. Newest first: 2026-10-20 3 EUR; 2026-10-20 10 USD; 2026-10-20 50 CNY (服务器费)"
    );
    let top = call(
        &member(json!({"action": "ranking"})),
        &venue(None, &[]),
        &context,
    );
    assert_eq!(
        text(&top),
        "Top sponsors (all time): 1. 小明 (qq:40001) 121.44 CNY; 2. 路人 80 CNY"
    );
    let nobody = call(
        &member(json!({"action": "person", "user": 5})),
        &venue(None, &[]),
        &context,
    );
    assert_eq!(text(&nobody), "No sponsorship records.");
    std::fs::remove_dir_all(dir).expect("删得掉");
}
