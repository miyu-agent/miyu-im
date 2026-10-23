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
