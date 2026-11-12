//! 占多大、整理（施工 I-230）：分约数照分量、整理以后硬盘上真的少了。

use super::super::{FILE, Said, Write, delete, prepared, written};
use super::*;

/// 临时目录里的一个库（同 `records::tests` 的 `fresh`）。
fn fresh(name: &str) -> (std::path::PathBuf, Connection) {
    let dir = std::env::temp_dir().join(format!("im-space-{}-{name}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("删得掉上一回的临时目录");
    }
    let connection = prepared(&dir.join(FILE)).expect("开得了");
    (dir, connection)
}

/// 场所 `venue` 里第 `n` 条、字是 `text`。
fn said(venue: &str, n: usize, text: &str) -> Write {
    Write::Said(Box::new(Said {
        venue: venue.to_string(),
        msg: n.to_string(),
        sender: "qq:40001".to_string(),
        name: None,
        at: i64::try_from(n).expect("不大"),
        text: text.to_string(),
        reply_to: None,
        mentions: Vec::new(),
        media: Vec::new(),
        mine: false,
    }))
}

#[test]
fn shares_follow_the_weights_and_zero_weights_get_nothing() {
    assert_eq!(shares(1000, &[3, 1]), [750, 250]);
    assert_eq!(shares(1000, &[0, 0]), [0, 0]);
    assert_eq!(shares(0, &[5]), [0]);
    assert!(
        shares(i64::MAX, &[i64::MAX, i64::MAX])
            .iter()
            .all(|one| *one > 0),
        "大数不溢出"
    );
}

#[test]
fn cleaning_then_compacting_gives_the_space_back_to_the_disk() {
    let (dir, connection) = fresh("compact");
    let long = "很长的一段聊天记录".repeat(40);
    for n in 0..2000 {
        written(&connection, &said("qq:1:group:31", n, &long)).expect("记得上");
    }
    written(&connection, &said("qq:1:group:32", 9000, "就一条")).expect("记得上");
    let path = dir.join(FILE);
    connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .expect("并得了");
    let full = on_disk(&path);
    let mut cleaning = super::super::connection(&path).expect("开得了");
    let scope = delete::Scope {
        venues: vec!["qq:1:group:31".into()],
        ..delete::Scope::default()
    };
    assert_eq!(delete::delete(&mut cleaning, &scope).expect("删得了"), 2000);
    assert!(
        on_disk(&path) >= full / 2,
        "光删不整理，文件不变小（这就是要整理的原因）"
    );
    compact(&cleaning).expect("整理得了");
    let after = on_disk(&path);
    assert!(after < full / 4, "整理以后硬盘上少了：{full} → {after}");
    // 约数：剩下的那一个场所分到库里用着的全部。
    let listed = super::super::read::venues(&cleaning).expect("列得出");
    assert_eq!(listed["venues"][0]["venue"], "qq:1:group:32");
    assert_eq!(
        listed["venues"][0]["bytes"].as_i64(),
        Some(used(&cleaning).expect("算得出"))
    );
    drop((connection, cleaning));
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}
