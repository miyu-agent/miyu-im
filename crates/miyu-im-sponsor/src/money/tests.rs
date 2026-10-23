//! 金额的写法、合计、排行榜。

use super::*;

fn entry(
    id: u64,
    user: Option<&str>,
    name: Option<&str>,
    converted: Option<f64>,
    at: i64,
) -> Entry {
    Entry {
        id,
        platform: "qq".to_string(),
        user: user.map(str::to_string),
        name: name.map(str::to_string),
        amount: converted.unwrap_or(10.0),
        currency: "CNY".to_string(),
        base: "CNY".to_string(),
        converted,
        rate: converted.map(|_| 1.0),
        at,
        note: None,
        by: None,
        venue: None,
    }
}

#[test]
fn amounts_drop_trailing_zeros() {
    assert_eq!(amount(50.0), "50");
    assert_eq!(amount(8.5), "8.5");
    assert_eq!(amount(357.204), "357.2");
    assert_eq!(cents(357.205_1), 357.21);
}

#[test]
fn totals_count_only_what_was_converted_to_the_base() {
    let mut other = entry(3, Some("qq:1"), None, Some(5.0), 3);
    other.base = "USD".to_string();
    let entries = [
        entry(1, Some("qq:1"), None, Some(50.0), 1),
        entry(2, Some("qq:1"), None, None, 2),
        other,
    ];
    let total = total(&entries, "CNY");
    assert_eq!(
        total,
        Total {
            sum: 50.0,
            counted: 1,
            pending: 2
        }
    );
}

#[test]
fn the_ranking_sums_per_sponsor_and_keeps_the_latest_name() {
    let entries = [
        entry(1, Some("qq:1"), Some("小明"), Some(50.0), 10),
        entry(2, None, Some("路人"), Some(80.0), 20),
        entry(3, Some("qq:1"), Some("明明"), Some(40.0), 30),
        entry(4, Some("qq:2"), None, None, 40),
    ];
    let rows = ranking(&entries, "CNY", SHOWN);
    let seen: Vec<(Sponsor, Option<String>, f64, usize)> = rows
        .into_iter()
        .map(|row| (row.sponsor, row.name, row.sum, row.count))
        .collect();
    assert_eq!(
        seen,
        [
            (
                Sponsor::Id("qq:1".to_string()),
                Some("明明".to_string()),
                90.0,
                2
            ),
            (
                Sponsor::Name("路人".to_string()),
                Some("路人".to_string()),
                80.0,
                1
            ),
        ],
        "号一样的合在一起、名字照最近的；只有昵称的单成一行；没换算的不进榜"
    );
    assert_eq!(ranking(&entries, "CNY", 1).len(), 1, "条数照调的一方");
}

#[test]
fn periods_start_at_the_local_month_and_year() {
    let zone = TimeZone::fixed(jiff::tz::offset(8));
    // 2026-10-20 12:00 东八区。
    let now = 1_792_468_800_000;
    assert_eq!(day(now, &zone), "2026-10-20");
    assert_eq!(
        Period::Month.since(now, &zone),
        Some(1_790_784_000_000),
        "10 月 1 日零点"
    );
    assert_eq!(
        Period::Year.since(now, &zone),
        Some(1_767_196_800_000),
        "1 月 1 日零点"
    );
    assert_eq!(Period::All.since(now, &zone), None);
}
