//! `miyu im records import <文件>`（施工 I-127，设计 04 第四节）：真的程序把中立写法的 JSON Lines 导进 im 状态目录里的库，
//! 说导了几样；重导只说跳过了几条；有一行不对的什么都不导、说第几行，退出码 1。

use rusqlite::Connection;

use crate::rules::clean;
use crate::support::spawning::{finished, program, text};
use crate::support::*;

/// 两条消息、一件撤回、一条判断。
const LINES: &str = concat!(
    r#"{"type":"message","venue":"qq:30003:group:31","msg":"101","sender":"qq:40001","name":"小明","at":1000,"text":"今天晚上一起打团本吗"}"#,
    "\n",
    r#"{"type":"message","venue":"qq:30003:group:31","msg":"102","sender":"qq:30003","at":2000,"text":"不去了","mine":true}"#,
    "\n",
    r#"{"type":"happening","venue":"qq:30003:group:31","at":3000,"kind":"recalled","actor":"qq:30003","msg":"102"}"#,
    "\n",
    r#"{"type":"judgment","venue":"qq:30003:group:31","msg":"101","at":1500,"outcome":"reply","why":{"route":"called"}}"#,
    "\n",
);

/// 库里有几条消息。
fn messages(root: &miyu_store::root::DataRoot) -> i64 {
    let db = miyu_im::dirs::state(root).join("records.db");
    Connection::open(db)
        .expect("开得了")
        .query_row("SELECT count(*) FROM messages", [], |row| row.get(0))
        .expect("数得出")
}

#[test]
fn the_program_imports_once_and_skips_on_the_second_run() {
    let (dir, root) = temp_root();
    let file = dir.join("records.jsonl");
    std::fs::write(&file, LINES).expect("写得进");
    let path = file.to_str().expect("UTF-8 路径");
    let first = finished(program(&root, &["records", "import", path]));
    assert_eq!(first.status.code(), Some(0), "{}", text(&first.stderr));
    assert_eq!(
        text(&first.stdout).trim(),
        format!("imported from {path}: 2 messages, 1 events, 1 judgments")
    );
    let again = finished(program(&root, &["records", "import", path]));
    assert_eq!(again.status.code(), Some(0), "{}", text(&again.stderr));
    assert!(
        text(&again.stdout).contains("skipped 4 already in the records and 0 whose number"),
        "{}",
        text(&again.stdout)
    );
    assert_eq!(messages(&root), 2);
    clean(&dir);
}

#[test]
fn a_wrong_line_imports_nothing_and_names_the_line() {
    let (dir, root) = temp_root();
    let file = dir.join("records.jsonl");
    let bad = LINES.replace(
        "qq:30003:group:31\",\"msg\":\"102\"",
        "qq:group:31\",\"msg\":\"102\"",
    );
    std::fs::write(&file, bad).expect("写得进");
    let path = file.to_str().expect("UTF-8 路径");
    let output = finished(program(&root, &["records", "import", path]));
    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(said.contains("nothing imported: 1 wrong lines"), "{said}");
    assert!(said.contains("line 2: 'qq:group:31' is not"), "{said}");
    assert_eq!(messages(&root), 0, "对的那几行也不导");
    clean(&dir);
}
