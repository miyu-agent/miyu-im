//! 导入聊天记录（施工 I-127）：三种行都写得进、搜得到；重导、和库里的重叠不多出来；编号撞上的留着库里的；有一行不对整份不导。

use std::path::PathBuf;

use super::*;
use crate::records::{FILE, prepared};

/// 临时目录里的一个库：`name` 是测试自己的名字（并着跑的测试各用各的目录）。
fn fresh(name: &str) -> (PathBuf, Connection) {
    let dir = std::env::temp_dir().join(format!("im-import-{}-{name}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("删得掉上一次留下的");
    }
    let connection = prepared(&dir.join(FILE)).expect("开得了");
    (dir, connection)
}

const VENUE: &str = "qq:30003:group:31";

/// 一份三种行都有的：两条消息（第二条后来撤回了）、一件撤回、一条判断。
fn lines() -> String {
    [
        r#"{"type":"message","venue":"qq:30003:group:31","msg":"101","sender":"qq:40001","name":"小明","at":1000,"text":"今天晚上一起打团本吗","mentions":["qq:40002"],"media":[{"kind":"image","id":"a.jpg"}]}"#,
        "",
        r#"{"type":"message","venue":"qq:30003:group:31","msg":"102","sender":"qq:30003","at":2000,"text":"不去了","reply_to":"101","mine":true}"#,
        r#"{"type":"happening","venue":"qq:30003:group:31","at":3000,"kind":"recalled","actor":"qq:30003","msg":"102"}"#,
        r#"{"type":"judgment","venue":"qq:30003:group:31","msg":"101","at":1500,"outcome":"reply","why":{"route":"called"}}"#,
    ]
    .join("\n")
}

fn count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("数得出")
}

#[test]
fn every_kind_of_line_is_written_and_the_text_is_searchable() {
    let (dir, mut connection) = fresh("kinds");
    let imported = import(&mut connection, lines().as_bytes())
        .expect("写得进")
        .expect("都对");
    assert_eq!(
        imported,
        Imported {
            messages: 2,
            happenings: 1,
            judgments: 1,
            duplicates: 0,
            conflicts: 0
        }
    );
    let found: String = connection
        .query_row(
            "SELECT m.msg FROM messages_text t JOIN messages m ON m.id = t.rowid WHERE messages_text MATCH ?1",
            ["打团本"],
            |row| row.get(0),
        )
        .expect("导进来的也搜得到");
    assert_eq!(found, "101");
    let recalled: (Option<i64>, Option<String>, bool) = connection
        .query_row(
            "SELECT recalled_at, recalled_by, mine FROM messages WHERE venue = ?1 AND msg = '102'",
            [VENUE],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("读得出");
    assert_eq!(recalled, (Some(3000), Some("qq:30003".to_string()), true));
    let why: String = connection
        .query_row("SELECT why FROM judgments WHERE msg = '101'", [], |row| {
            row.get(0)
        })
        .expect("读得出");
    assert_eq!(why, r#"{"route":"called"}"#);
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}

#[test]
fn importing_twice_adds_nothing_and_a_clashing_number_keeps_what_is_there() {
    let (dir, mut connection) = fresh("twice");
    import(&mut connection, lines().as_bytes())
        .expect("写得进")
        .expect("都对");
    let again = import(&mut connection, lines().as_bytes())
        .expect("写得进")
        .expect("都对");
    assert_eq!(
        again,
        Imported {
            duplicates: 4,
            ..Imported::default()
        }
    );
    for (table, rows) in [("messages", 2), ("happenings", 1), ("judgments", 1)] {
        assert_eq!(count(&connection, table), rows, "{table}");
    }
    // 同一个编号、别人在别的时刻说的：平台编号撞上了，留着库里的。
    let clash = r#"{"type":"message","venue":"qq:30003:group:31","msg":"101","sender":"qq:49999","at":9000,"text":"另一条"}"#;
    let clashed = import(&mut connection, clash.as_bytes())
        .expect("写得进")
        .expect("都对");
    assert_eq!(
        clashed,
        Imported {
            conflicts: 1,
            ..Imported::default()
        }
    );
    let text: String = connection
        .query_row("SELECT text FROM messages WHERE msg = '101'", [], |row| {
            row.get(0)
        })
        .expect("读得出");
    assert_eq!(text, "今天晚上一起打团本吗");
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}

#[test]
fn one_wrong_line_rejects_the_whole_file() {
    let (dir, mut connection) = fresh("rejected");
    let bad = [
        r#"{"type":"message","venue":"qq:30003:group:31","msg":"101","sender":"qq:40001","at":1000,"text":"对的"}"#,
        r#"{"type":"message","venue":"qq:group:31","msg":"102","sender":"qq:40001","at":1000,"text":"老写法"}"#,
        r#"{"type":"happening","venue":"qq:30003:group:31","at":1000,"kind":"banned"}"#,
        r#"{"type":"judgment","venue":"qq:30003:group:31","msg":"101","at":1000,"outcome":"maybe","why":null}"#,
        r#"{"type":"message","venue":"qq:30003:group:31","msg":"","sender":"qq:40001","at":1000,"text":"没编号"}"#,
        r#"{"type":"message","venue":"qq:30003:group:31","msg":"103","sender":"qq:40001","at":0,"text":"没时刻"}"#,
        r#"{"type":"message","venue":"qq:30003:group:31","msg":"104","sender":"qq:40001","at":1000,"text":"多一格","color":"red"}"#,
        "not json",
    ]
    .join("\n");
    let Err(Rejected(problems)) = import(&mut connection, bad.as_bytes()).expect("没碰库")
    else {
        panic!("有不对的行要整份不导");
    };
    let shapes: Vec<(usize, &str)> = problems
        .iter()
        .map(|(line, problem)| {
            let shape = match problem {
                Problem::Json(_) => "json",
                Problem::Venue(_) => "venue",
                Problem::Kind(_) => "kind",
                Problem::Outcome(_) => "outcome",
                Problem::Empty(field) => field,
            };
            (*line, shape)
        })
        .collect();
    assert_eq!(
        shapes,
        [
            (2, "venue"),
            (3, "kind"),
            (4, "outcome"),
            (5, "msg"),
            (6, "at"),
            (7, "json"),
            (8, "json")
        ]
    );
    assert_eq!(count(&connection, "messages"), 0, "对的那一行也不导");
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}
