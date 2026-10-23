//! 聊天记录的库（施工 C-1 上）：建表、记消息（同一条只记一次）、撤回标记、事件、全文搜索中文子串。

use serde_json::json;

use super::*;

/// 临时目录里的一个库：`name` 是测试自己的名字（并着跑的测试各用各的目录）。
fn fresh(name: &str) -> (PathBuf, Connection) {
    let dir = std::env::temp_dir().join(format!("im-records-{}-{name}", std::process::id()));
    let path = dir.join(FILE);
    let connection = prepared(&path).expect("开得了");
    (dir, connection)
}

fn said(msg: &str, text: &str) -> Write {
    Write::Said(Box::new(Said {
        venue: "qq:group:31".to_string(),
        msg: msg.to_string(),
        sender: "qq:40001".to_string(),
        name: Some("小明".to_string()),
        at: 1_000,
        text: text.to_string(),
        reply_to: None,
        mentions: vec!["qq:40002".to_string()],
        media: vec![json!({"kind": "image", "id": "abc.jpg"})],
        mine: false,
    }))
}

#[test]
fn a_message_is_kept_once_and_found_by_a_chinese_substring() {
    let (dir, connection) = fresh("substring");
    written(&connection, &said("101", "今天晚上一起打团本吗")).expect("记得上");
    written(&connection, &said("101", "重发的同一条")).expect("重发的不报错");
    written(&connection, &said("102", "不去了")).expect("记得上");
    let count: i64 = connection
        .query_row("SELECT count(*) FROM messages", [], |row| row.get(0))
        .expect("数得出");
    assert_eq!(count, 2, "同一个编号只记一次");
    let found: String = connection
        .query_row(
            "SELECT m.msg FROM messages_text t JOIN messages m ON m.id = t.rowid WHERE messages_text MATCH ?1",
            ["打团本"],
            |row| row.get(0),
        )
        .expect("搜得到");
    assert_eq!(found, "101");
    let media: String = connection
        .query_row("SELECT media FROM messages WHERE msg = '101'", [], |row| {
            row.get(0)
        })
        .expect("读得出");
    assert_eq!(
        media, r#"[{"id":"abc.jpg","kind":"image"}]"#,
        "只记占位和编号"
    );
    // 先关掉连接（Windows 上开着的文件删不掉）。
    drop(connection);
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}

#[test]
fn a_recall_marks_the_message_and_is_an_event() {
    let (dir, connection) = fresh("recall");
    written(&connection, &said("101", "发错了")).expect("记得上");
    let recalled = Happening {
        venue: "qq:group:31".to_string(),
        at: 2_000,
        kind: RECALLED,
        target: Some("qq:40001".to_string()),
        actor: Some("qq:40001".to_string()),
        seconds: None,
        msg: Some("101".to_string()),
    };
    written(&connection, &Write::Happened(recalled)).expect("记得上");
    let (at, by, text): (i64, String, String) = connection
        .query_row(
            "SELECT recalled_at, recalled_by, text FROM messages WHERE msg = '101'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("读得出");
    assert_eq!(
        (at, by.as_str(), text.as_str()),
        (2_000, "qq:40001", "发错了"),
        "撤回不删，标上"
    );
    let kinds: i64 = connection
        .query_row(
            "SELECT count(*) FROM happenings WHERE kind = 'recalled'",
            [],
            |row| row.get(0),
        )
        .expect("数得出");
    assert_eq!(kinds, 1);
    // 先关掉连接（Windows 上开着的文件删不掉）。
    drop(connection);
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}

#[test]
fn the_platform_time_is_seconds_and_missing_means_now() {
    assert_eq!(at(1_700_000_000), 1_700_000_000_000);
    assert!(at(0) > 1_700_000_000_000);
}

/// 场所 31 里号是 `sender` 的人第 `seconds` 秒说的第 `msg` 条。
fn said_at(msg: &str, sender: &str, name: &str, seconds: i64, text: &str) -> Write {
    Write::Said(Box::new(Said {
        venue: "qq:group:31".to_string(),
        msg: msg.to_string(),
        sender: sender.to_string(),
        name: Some(name.to_string()),
        at: seconds * 1000,
        text: text.to_string(),
        reply_to: None,
        mentions: Vec::new(),
        media: Vec::new(),
        mine: false,
    }))
}

#[test]
fn a_page_goes_back_with_judgements_and_events_in_time_order() {
    let (dir, connection) = fresh("page");
    for n in 1..=5 {
        written(
            &connection,
            &said_at(&n.to_string(), "qq:1", "小明", n * 10, &format!("第{n}句")),
        )
        .expect("记得上");
    }
    let judged = Write::Judged {
        venue: "qq:group:31".to_string(),
        msg: "5".to_string(),
        outcome: "record".to_string(),
        why: json!({"reason": "quiet"}),
    };
    written(&connection, &judged).expect("记得上");
    let muted = Happening {
        venue: "qq:group:31".to_string(),
        at: 45_000,
        kind: "muted",
        target: Some("qq:1".to_string()),
        actor: Some("qq:2".to_string()),
        seconds: Some(60),
        msg: None,
    };
    written(&connection, &Write::Happened(muted)).expect("记得上");
    let first = read::page(&connection, "qq:group:31", None, 2).expect("读得出");
    let kinds: Vec<String> = first["items"]
        .as_array()
        .expect("有")
        .iter()
        .map(|item| {
            format!(
                "{}:{}",
                item["type"].as_str().unwrap_or_default(),
                item["text"]
                    .as_str()
                    .or(item["kind"].as_str())
                    .unwrap_or_default()
            )
        })
        .collect();
    assert_eq!(
        kinds,
        ["message:第4句", "event:muted", "message:第5句"],
        "从旧到新，事件夹在中间"
    );
    assert_eq!(first["more"], true);
    assert_eq!(
        first["items"][2]["judged"],
        json!({"outcome": "record", "why": {"reason": "quiet"}})
    );
    let second =
        read::page(&connection, "qq:group:31", first["before"].as_i64(), 10).expect("读得出");
    let texts: Vec<&str> = second["items"]
        .as_array()
        .expect("有")
        .iter()
        .filter_map(|item| item["text"].as_str())
        .collect();
    assert_eq!(texts, ["第1句", "第2句", "第3句"]);
    assert_eq!(second["more"], false);
    drop(connection);
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}

#[test]
fn search_finds_words_people_and_times_across_venues() {
    let (dir, connection) = fresh("search");
    written(
        &connection,
        &said_at("1", "qq:10001", "小明", 10, "今晚打团本吗"),
    )
    .expect("记得上");
    written(&connection, &said_at("2", "qq:10002", "小林", 20, "打")).expect("记得上");
    written(
        &connection,
        &said_at("3", "qq:10002", "小林", 30, "不去团本了"),
    )
    .expect("记得上");
    let found = |query: read::Query| -> Vec<String> {
        read::search(&connection, &query).expect("搜得出")["items"]
            .as_array()
            .expect("有")
            .iter()
            .map(|item| item["msg"].as_str().unwrap_or_default().to_string())
            .collect()
    };
    let text = |text: &str| read::Query {
        text: text.to_string(),
        limit: 50,
        ..read::Query::default()
    };
    assert_eq!(found(text("团本")), ["3", "1"], "两个字照子串，新的在前");
    assert_eq!(found(text("打团本")), ["1"], "三个字照全文索引");
    assert_eq!(
        found(read::Query {
            sender: Some("10002".to_string()),
            ..text("")
        }),
        ["3", "2"],
        "按号"
    );
    assert_eq!(
        found(read::Query {
            sender: Some("小明".to_string()),
            ..text("")
        }),
        ["1"],
        "按名字"
    );
    assert_eq!(
        found(read::Query {
            from: Some(15_000),
            to: Some(25_000),
            ..text("")
        }),
        ["2"],
        "按时间段"
    );
    assert_eq!(found(text("100%")), Vec::<String>::new(), "百分号照字面");
    drop(connection);
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}

#[test]
fn delete_takes_a_venue_a_span_or_a_person_and_never_everything_by_accident() {
    let (dir, mut connection) = fresh("delete");
    for (msg, sender, seconds) in [("1", "qq:1", 10), ("2", "qq:2", 20), ("3", "qq:1", 30)] {
        written(&connection, &said_at(msg, sender, "x", seconds, "话")).expect("记得上");
    }
    let count = |connection: &Connection| -> i64 {
        connection
            .query_row("SELECT count(*) FROM messages", [], |row| row.get(0))
            .expect("数得出")
    };
    assert_eq!(
        delete::delete(&mut connection, &delete::Scope::default()).expect("删得了"),
        0,
        "没写条件不删"
    );
    let person = delete::Scope {
        sender: Some("qq:1".to_string()),
        ..delete::Scope::default()
    };
    assert_eq!(delete::delete(&mut connection, &person).expect("删得了"), 2);
    let span = delete::Scope {
        from: Some(15_000),
        to: Some(25_000),
        ..delete::Scope::default()
    };
    assert_eq!(delete::delete(&mut connection, &span).expect("删得了"), 1);
    assert_eq!(count(&connection), 0);
    written(&connection, &said_at("4", "qq:3", "x", 40, "话")).expect("记得上");
    let all = delete::Scope {
        all: true,
        ..delete::Scope::default()
    };
    assert_eq!(delete::delete(&mut connection, &all).expect("删得了"), 1);
    drop(connection);
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}
