//! 账本的读写。

use super::*;

fn dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("im-sponsor-{}-{name}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).expect("建得了");
    dir
}

fn new(user: Option<&str>, amount: f64) -> New {
    New {
        platform: "qq".to_string(),
        user: user.map(str::to_string),
        name: Some("小明".to_string()),
        amount,
        currency: "CNY".to_string(),
        base: "CNY".to_string(),
        converted: Some(amount),
        rate: Some(1.0),
        at: 1_000,
        note: None,
        by: Some("qq:9".to_string()),
        venue: Some("qq:group:1".to_string()),
    }
}

#[test]
fn entries_get_numbers_and_survive_a_reopen() {
    let dir = dir("reopen");
    let path = dir.join("sponsors.json");
    let mut book = Book::open(&path).expect("没有的是空的");
    assert!(book.entries().is_empty());
    assert_eq!(book.add(new(Some("qq:1"), 50.0)).expect("记得上").id, 1);
    assert_eq!(book.add(new(None, 8.5)).expect("记得上").id, 2);
    let again = Book::open(&path).expect("读得出");
    assert_eq!(again.entries().len(), 2);
    assert_eq!(again.entries()[1].user, None, "只有昵称的也记");
    assert!(!dir.join("sponsors.json.tmp").exists(), "临时文件换过去了");
    std::fs::remove_dir_all(dir).expect("删得掉");
}

#[test]
fn a_broken_book_is_not_overwritten() {
    let dir = dir("broken");
    let path = dir.join("sponsors.json");
    std::fs::write(&path, "{not json").expect("写得进");
    assert!(Book::open(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path).expect("还在"), "{not json");
    std::fs::remove_dir_all(dir).expect("删得掉");
}
