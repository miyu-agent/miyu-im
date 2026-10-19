use std::sync::Arc;

use super::{Count, Counters, Day, today};
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
        counters.bump_on(count, "2026-10-19");
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
    counters.bump_on(Count::Received, "2026-10-20");
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
    if std::fs::remove_dir_all(&dir).is_err() {
        // 删不掉就留在临时目录里。
    }
}
