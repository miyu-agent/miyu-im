//! 管理记录的读写（施工 Q-2）。

use super::*;

fn muted(at: u64, user: i64) -> Record {
    Record {
        at,
        action: Action::Mute,
        user,
        name: Some("小明".to_string()),
        by: By::Ai,
        seconds: Some(600),
        reason: Some("刷屏".to_string()),
        block: false,
        verdict: None,
        comment: None,
    }
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("miyu-im-records-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("删得了");
    }
    dir
}

#[tokio::test]
async fn records_are_appended_per_group_and_the_oldest_go_past_keep() {
    let store = temp("keep");
    for at in 1..=4 {
        append(
            store.clone(),
            31,
            muted(at, 100 + i64::try_from(at).expect("小")),
            3,
        )
        .await;
    }
    append(store.clone(), 32, muted(9, 7), 3).await;
    let kept: Vec<u64> = read(&store, 31).iter().map(|record| record.at).collect();
    assert_eq!(kept, [2, 3, 4], "只留最后三条，旧的在前");
    assert_eq!(read(&store, 32), [muted(9, 7)], "一个群一份");
    assert!(read(&store, 33).is_empty(), "没有的是空的");
    std::fs::remove_dir_all(&store).expect("删得了");
}

#[tokio::test]
async fn broken_lines_are_skipped_and_dropped_on_the_next_write() {
    let store = temp("broken");
    append(store.clone(), 31, muted(1, 1), 10).await;
    let path = file(&store, 31);
    let mut text = std::fs::read_to_string(&path).expect("读得出");
    text.push_str("{not json\n");
    std::fs::write(&path, text).expect("写得进");
    assert_eq!(read(&store, 31).len(), 1);
    append(store.clone(), 31, muted(2, 2), 10).await;
    let text = std::fs::read_to_string(&path).expect("读得出");
    assert_eq!(text.lines().count(), 2, "{text}");
    std::fs::remove_dir_all(&store).expect("删得了");
}

#[test]
fn a_record_is_written_with_only_the_fields_it_has() {
    let kick = Record {
        at: 5,
        action: Action::Kick,
        user: 8,
        name: None,
        by: By::Admin {
            user: 9,
            name: Some("阿杰".to_string()),
        },
        seconds: None,
        reason: None,
        block: true,
        verdict: None,
        comment: None,
    };
    assert_eq!(
        serde_json::to_value(&kick).expect("写得出"),
        serde_json::json!({"at": 5, "action": "kick", "user": 8, "by": {"kind": "admin", "user": 9, "name": "阿杰"}, "block": true})
    );
    let back: Record =
        serde_json::from_value(serde_json::to_value(&kick).expect("写得出")).expect("读得回");
    assert_eq!(back, kick);
}
