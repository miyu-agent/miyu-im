//! 统计页的算法（施工 I-14）。

use serde_json::json;

use super::*;
use crate::counters::{Daily, Day};

/// 选中连接 `id`。
fn scope(id: &str) -> Scope {
    Scope::of(&json!({"connection": id}))
        .expect("写法对")
        .expect("选了一个")
}

#[test]
fn only_the_chosen_connections_venue_sessions_count() {
    let rows = json!([
        {"venue": "qq:group:777", "day": "2026-10-20", "purpose": "turn", "model": "dev/m", "requests": 2, "usage": {"cache_read": 800, "uncached": 200, "cache_write": 0, "output": 50}},
        {"venue": "qq:private:5", "day": "2026-10-19", "purpose": "judge", "model": "dev/s", "requests": 3, "usage": {"cache_read": 0, "uncached": 300, "cache_write": 0, "output": 30}},
        {"venue": null, "day": "2026-10-20", "purpose": "turn", "model": "dev/m", "requests": 9, "usage": {"cache_read": 9, "uncached": 9, "cache_write": 9, "output": 9}},
        {"venue": "tg:group:1", "day": "2026-10-20", "purpose": "turn", "model": "dev/m", "requests": 9, "usage": {"uncached": 9}},
    ]);
    let before =
        json!([{"venue": "qq:group:777", "requests": 1, "usage": {"uncached": 100, "output": 10}}]);
    let qq = scope("qq:10001");
    let shown = token(&ours(&rows, Some(&qq)), &ours(&before, Some(&qq)), 4);
    assert_eq!(shown["total"], 1380);
    assert_eq!(shown["prev_total"], 110);
    assert_eq!(shown["requests"], 5);
    assert_eq!(shown["per_reply"], 345);
    let rate = shown["hit_rate"].as_f64().expect("有");
    assert!((rate - 800.0 / 1300.0).abs() < 1e-9, "{rate}");
    assert_eq!(shown["parts"], json!([800, 500, 0, 80]));
    assert_eq!(
        shown["days"],
        json!([{"day": "2026-10-19", "parts": [0, 300, 0, 30]}, {"day": "2026-10-20", "parts": [800, 200, 0, 50]}])
    );
    assert_eq!(
        shown["purposes"][0],
        json!({"name": "turn", "tokens": 1050, "requests": 2})
    );
    assert_eq!(shown["venues"][1]["name"], "qq:private:5");
    assert_eq!(token(&[], &[], 0)["per_reply"], serde_json::Value::Null);
    // 「全部」：场所会话都算，不是场所会话的照旧不算。
    assert_eq!(ours(&rows, None).len(), 3);
}

#[test]
fn messages_add_up_the_days() {
    let day = |received, replied| Daily {
        total: Day {
            received,
            replied,
            recorded: 1,
            undelivered: 0,
        },
        venues: [(
            "qq:group:777".to_string(),
            Day {
                received,
                ..Day::default()
            },
        )]
        .into(),
        reasons: [("called".to_string(), 2)].into(),
        hours: {
            let mut hours = vec![0; 24];
            hours[9] = received;
            hours
        },
        speakers: std::collections::BTreeMap::new(),
        connection_hours: std::collections::BTreeMap::new(),
    };
    let now = vec![
        ("2026-10-19".to_string(), day(3, 1)),
        ("2026-10-20".to_string(), day(5, 2)),
    ];
    let before = vec![("2026-10-18".to_string(), day(1, 1))];
    let shown = messages(&now, &before, None);
    assert_eq!(
        shown["now"],
        json!({"received": 8, "replied": 3, "recorded": 2, "undelivered": 0})
    );
    assert_eq!(shown["prev"]["received"], 1);
    assert_eq!(
        shown["venues"],
        json!([{"name": "qq:group:777", "received": 8}])
    );
    assert_eq!(shown["reasons"], json!({"called": 4}));
    assert_eq!(shown["hours"][9], 8);
    assert_eq!(
        shown["days"][1],
        json!({"day": "2026-10-20", "received": 5, "replied": 2})
    );
    let old = vec![("2026-10-01".to_string(), Daily::default())];
    assert_eq!(
        messages(&old, &[], None)["hours"][0],
        0,
        "以前写的没有小时也照算"
    );
}

#[test]
fn a_chosen_connection_counts_only_its_venues() {
    let venue = |received| Day {
        received,
        replied: 1,
        ..Day::default()
    };
    let hours = |at: usize, count| {
        let mut hours = vec![0; 24];
        hours[at] = count;
        hours
    };
    let daily = Daily {
        total: Day {
            received: 7,
            replied: 2,
            ..Day::default()
        },
        venues: [
            ("qq:group:777".to_string(), venue(3)),
            ("tg:group:-100".to_string(), venue(4)),
        ]
        .into(),
        reasons: [("called".to_string(), 2)].into(),
        hours: hours(9, 7),
        connection_hours: [
            ("qq".to_string(), hours(9, 3)),
            ("tg".to_string(), hours(9, 4)),
        ]
        .into(),
        ..Daily::default()
    };
    let now = vec![("2026-10-20".to_string(), daily)];
    let tg = scope("tg:7890");
    let shown = messages(&now, &[], Some(&tg));
    assert_eq!(shown["now"]["received"], 4, "{shown}");
    assert_eq!(shown["now"]["replied"], 1, "{shown}");
    assert_eq!(
        shown["venues"],
        json!([{"name": "tg:group:-100", "received": 4}])
    );
    assert_eq!(shown["hours"][9], 4);
    assert_eq!(shown["days"][0]["received"], 4);
    assert_eq!(shown["reasons"], json!({}), "原因不分场所，选了连接的不给");
    let all = messages(&now, &[], None);
    assert_eq!(all["now"]["received"], 7);
    assert_eq!(all["hours"][9], 7);
}
