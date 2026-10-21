use std::sync::Arc;

use super::{Count, Counters, Daily, Day, today};
use crate::changed::Changed;

fn counters(dir: &std::path::Path) -> Counters {
    Counters::load(dir.to_path_buf(), Arc::new(Changed::new()))
}

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("miyu-im-counters-{name}-{}", std::process::id()));
    if std::fs::remove_dir_all(&dir).is_err() {
        // 没有就是没有。
    }
    dir
}

#[test]
fn each_count_goes_to_its_own_number_and_a_new_day_starts_at_zero() {
    let dir = temp("days");
    let counters = counters(&dir);
    for count in [
        Count::Received,
        Count::Received,
        Count::Replied,
        Count::Recorded,
        Count::Undelivered,
    ] {
        counters.bump_on(count, "qq:group:1", "2026-10-19");
    }
    assert_eq!(
        counters.on("2026-10-19"),
        Day {
            received: 2,
            replied: 1,
            recorded: 1,
            undelivered: 1
        }
    );
    assert_eq!(counters.on("2026-10-20"), Day::default(), "别的一天当零");
    counters.bump_on(Count::Received, "qq:group:1", "2026-10-20");
    assert_eq!(
        counters.on("2026-10-20"),
        Day {
            received: 1,
            ..Day::default()
        }
    );
    assert_eq!(counters.on("2026-10-19"), Day::default(), "跨了天的从零数");
}

#[test]
fn each_venue_has_its_own_numbers_and_some_add_up() {
    let dir = temp("venues");
    let counters = counters(&dir);
    let day = "2026-10-20";
    counters.bump_on(Count::Received, "qq:group:1", day);
    counters.bump_on(Count::Received, "qq:group:1", day);
    counters.bump_on(Count::Replied, "qq:group:1", day);
    counters.bump_on(Count::Received, "qq:group:2", day);
    counters.bump_on(Count::Recorded, "qq:group:2", day);
    counters.bump_on(Count::Received, "qq:private:3", day);
    let one = counters.of_on(|venue| venue == "qq:group:1", day);
    assert_eq!(
        one,
        Day {
            received: 2,
            replied: 1,
            ..Day::default()
        }
    );
    let groups = counters.of_on(|venue| venue.starts_with("qq:group:"), day);
    assert_eq!(
        groups,
        Day {
            received: 3,
            replied: 1,
            recorded: 1,
            undelivered: 0
        }
    );
    assert_eq!(counters.on(day).received, 4, "合计照数");
    assert_eq!(
        counters.of_on(|_| true, "2026-10-21"),
        Day::default(),
        "别的一天当零"
    );
    counters.bump_on(Count::Received, "qq:group:2", "2026-10-21");
    assert_eq!(
        counters.of_on(|venue| venue == "qq:group:1", "2026-10-21"),
        Day::default(),
        "跨了天的从零数"
    );
}

#[test]
fn todays_file_is_read_back() {
    let dir = temp("file");
    std::fs::create_dir_all(&dir).expect("建得了目录");
    let written = Day {
        received: 7,
        replied: 3,
        recorded: 4,
        undelivered: 1,
    };
    std::fs::write(
        dir.join(format!("{}.json", today())),
        serde_json::to_vec(&written).expect("写得成"),
    )
    .expect("写得进");
    std::fs::write(dir.join("2000-01-01.json"), b"{\"received\": 99}").expect("写得进");
    assert_eq!(counters(&dir).today(), written, "照今天那一份接着数");
    std::fs::write(dir.join(format!("{}.json", today())), b"not json").expect("写得进");
    assert_eq!(counters(&dir).today(), Day::default(), "读不懂的从零数");
    // 带场所的那一份（施工 I-7 上补）：合计、场所的都读回来。
    let with = r#"{"received": 2, "venues": {"qq:group:1": {"received": 2}}}"#;
    std::fs::write(dir.join(format!("{}.json", today())), with).expect("写得进");
    let again = counters(&dir);
    assert_eq!(again.today().received, 2);
    assert_eq!(again.today_of(|venue| venue == "qq:group:1").received, 2);
    if std::fs::remove_dir_all(&dir).is_err() {
        // 删不掉就留在临时目录里。
    }
}

#[test]
fn the_ranking_adds_up_days_for_one_group() {
    // 施工 I-20：几天加起来，多的在前，一样多的照编号；别的群的不算。
    let day = |pairs: &[(&str, &str, u64)]| {
        let mut daily = Daily::default();
        for (venue, person, count) in pairs {
            daily
                .speakers
                .entry((*venue).to_string())
                .or_default()
                .insert((*person).to_string(), *count);
        }
        daily
    };
    let days = [
        (
            "2026-10-20".to_string(),
            day(&[("g", "qq:1", 3), ("g", "qq:2", 5), ("h", "qq:9", 99)]),
        ),
        (
            "2026-10-21".to_string(),
            day(&[("g", "qq:1", 2), ("g", "qq:3", 5)]),
        ),
    ];
    assert_eq!(
        super::ranking(&days, "g"),
        [
            ("qq:1".to_string(), 5),
            ("qq:2".to_string(), 5),
            ("qq:3".to_string(), 5)
        ]
    );
    assert!(super::ranking(&days, "none").is_empty());
}
