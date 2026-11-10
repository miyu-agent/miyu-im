//! 每人的用量账本：平分、去重、按人按时刻加、太早的不记、清掉过了期的。

use std::path::PathBuf;

use miyu_client::ids::ExternalId;

use super::*;

/// 一个空的临时目录。
fn dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("im-spent-{}-{name}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("删得掉上一回留下的");
    }
    dir
}

fn person(id: &str) -> ExternalId {
    ExternalId::parse(id).expect("合写法")
}

/// 库 `dir` 里 `person` 从 `from` 起记了多少。
fn since(dir: &std::path::Path, person: &str, from: i64) -> i64 {
    rusqlite::Connection::open(dir.join(FILE))
        .and_then(|db| {
            db.query_row(
                "SELECT COALESCE(SUM(tokens), 0) FROM spent WHERE person = ?1 AND at >= ?2",
                rusqlite::params![person, from],
                |row| row.get(0),
            )
        })
        .unwrap_or(-1)
}

/// 此刻：随便一个够大的毫秒数。
const NOW: i64 = 1_800_000_000_000;

fn charge(seq: u64, at: i64, tokens: u64) -> Charge<'static> {
    Charge {
        session: "s1",
        seq,
        venue: "qq:group:1",
        at,
        tokens,
        cost: None,
    }
}

#[test]
fn shares_split_evenly_and_the_rest_goes_to_the_first() {
    assert_eq!(shares(10, 3), [4, 3, 3]);
    assert_eq!(shares(9, 3), [3, 3, 3]);
    assert_eq!(shares(1, 2), [1, 0]);
    assert!(shares(10, 0).is_empty());
}

#[test]
fn one_request_split_between_two_is_counted_once_per_person_even_when_replayed() {
    let dir = dir("split");
    let spent = Spent::open(&dir.join(FILE));
    let (lin, jie) = (person("qq:1"), person("qq:2"));
    spent.charge(
        &charge(5, NOW - 1000, 101),
        &[lin.clone(), jie.clone()],
        NOW,
    );
    // 桥重起照日志从头补：同一条再来一次。
    spent.charge(
        &charge(5, NOW - 1000, 101),
        &[lin.clone(), jie.clone()],
        NOW,
    );
    spent.charge(&charge(7, NOW - 500, 40), std::slice::from_ref(&lin), NOW);
    assert_eq!(since(&dir, "qq:1", NOW - 2000), 51 + 40);
    assert_eq!(since(&dir, "qq:2", NOW - 2000), 50);
    assert_eq!(since(&dir, "qq:1", NOW - 600), 40, "只算窗口里的");
    assert_eq!(since(&dir, "qq:3", NOW - 2000), 0, "没用过的是 0");
    drop(spent);
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}

#[test]
fn nobody_to_pay_zero_tokens_and_too_old_are_not_written() {
    let dir = dir("skipped");
    let spent = Spent::open(&dir.join(FILE));
    let lin = person("qq:1");
    spent.charge(&charge(1, NOW, 100), &[], NOW);
    spent.charge(&charge(2, NOW, 0), std::slice::from_ref(&lin), NOW);
    spent.charge(
        &charge(3, NOW - KEEP_MS - 1, 100),
        std::slice::from_ref(&lin),
        NOW,
    );
    assert_eq!(since(&dir, "qq:1", 0), 0);
    drop(spent);
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}

#[test]
fn old_rows_are_pruned_on_the_next_charge_an_hour_later() {
    let dir = dir("pruned");
    let spent = Spent::open(&dir.join(FILE));
    let lin = person("qq:1");
    spent.charge(&charge(1, NOW, 100), std::slice::from_ref(&lin), NOW);
    let later = NOW + KEEP_MS + HOUR_MS;
    spent.charge(&charge(2, later, 7), std::slice::from_ref(&lin), later);
    assert_eq!(since(&dir, "qq:1", 0), 7, "过了期的清掉了");
    drop(spent);
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}

#[test]
fn a_ledger_that_cannot_open_records_nothing_and_does_not_fail() {
    let dir = dir("broken");
    std::fs::create_dir_all(dir.join(FILE)).expect("占住库的位置");
    let spent = Spent::open(&dir.join(FILE));
    spent.charge(&charge(1, NOW, 100), &[person("qq:1")], NOW);
    assert!(dir.join(FILE).is_dir(), "占着的没动");
    drop(spent);
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}

#[test]
fn the_page_lists_people_by_the_last_five_hours_then_the_month() {
    let dir = dir("top");
    let path = dir.join(FILE);
    assert!(top(&path, NOW, 10).expect("读得了").is_empty(), "还没有库");
    assert!(!path.exists(), "只读地开，不建");
    let spent = Spent::open(&path);
    let hour = 60 * 60 * 1000;
    spent.charge(&charge(1, NOW - hour, 10), &[person("qq:1")], NOW);
    spent.charge(&charge(2, NOW - 6 * hour, 500), &[person("qq:1")], NOW);
    spent.charge(&charge(3, NOW - hour, 30), &[person("qq:2")], NOW);
    spent.charge(
        &charge(4, NOW - 10 * 24 * hour, 900),
        &[person("qq:3")],
        NOW,
    );
    drop(spent);
    let listed = top(&path, NOW, 10).expect("读得了");
    assert_eq!(
        listed,
        [
            ("qq:2".to_string(), [30, 30, 30]),
            ("qq:1".to_string(), [10, 510, 510]),
            ("qq:3".to_string(), [0, 0, 900]),
        ]
    );
    assert_eq!(top(&path, NOW, 1).expect("读得了").len(), 1);
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}

#[test]
fn amounts_are_split_kept_by_day_and_currency_and_old_rows_count_as_unpriced() {
    // 施工 I-205：金额跟着用量平分、连币种记；照「日子、币种」加好交出去，没有金额的请求数出来。
    let dir = dir("costs");
    // I-205 以前建的库：没有金额的两列，里头有一行老的。打开时补上列，老的行照「没记录金额」读。
    std::fs::create_dir_all(&dir).expect("建得了");
    let old = rusqlite::Connection::open(dir.join(FILE)).expect("开得了");
    old.execute_batch(
        "CREATE TABLE spent (session TEXT NOT NULL, seq INTEGER NOT NULL, person TEXT NOT NULL,
            venue TEXT NOT NULL, at INTEGER NOT NULL, tokens INTEGER NOT NULL, PRIMARY KEY (session, seq, person));",
    )
    .expect("建得了");
    old.execute(
        "INSERT INTO spent VALUES ('s0', 1, 'qq:1', 'qq:group:1', ?1, 10)",
        [NOW - 1000],
    )
    .expect("写得进");
    drop(old);
    let spent = Spent::open(&dir.join(FILE));
    let (lin, jie) = (person("qq:1"), person("qq:2"));
    let priced = |seq, at, cost| Charge {
        cost,
        ..charge(seq, at, 100)
    };
    let day_ms = 24 * 60 * 60 * 1000;
    spent.charge(
        &priced(5, NOW - 1000, Some((0.5, "USD"))),
        &[lin.clone(), jie.clone()],
        NOW,
    );
    spent.charge(
        &priced(6, NOW - 900, Some((0.25, "USD"))),
        std::slice::from_ref(&lin),
        NOW,
    );
    spent.charge(
        &priced(7, NOW - 2 * day_ms, Some((3.0, "CNY"))),
        std::slice::from_ref(&lin),
        NOW,
    );
    // 核心算不出价的一次。
    spent.charge(&priced(8, NOW - 800, None), std::slice::from_ref(&lin), NOW);
    let people = ["qq:1".to_string(), "qq:2".to_string(), "qq:9".to_string()];
    let found = costs(&dir.join(FILE), &people, NOW).expect("读得出");
    let today = day(NOW);
    let lin = &found["qq:1"];
    assert_eq!(
        lin[0].parts,
        [(today.clone(), "USD".to_string(), 0.5)],
        "5 小时：平分来的 0.25 加自己的 0.25"
    );
    assert_eq!(lin[0].unpriced, 2, "老的一行、算不出价的一次");
    assert_eq!(
        lin[1].parts,
        [
            (day(NOW - 2 * day_ms), "CNY".to_string(), 3.0),
            (today.clone(), "USD".to_string(), 0.5)
        ],
        "7 天：两天前的那一笔照它的日子、币种另放"
    );
    assert_eq!(found["qq:2"][2].parts, [(today, "USD".to_string(), 0.25)]);
    assert_eq!(found["qq:2"][2].unpriced, 0);
    assert_eq!(found["qq:9"], <[Costs; 3]>::default(), "没用过的是空的");
}
