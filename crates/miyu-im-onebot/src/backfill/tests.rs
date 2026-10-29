use miyu_im::chat::Id;
use serde_json::{Value, json};

use super::{COUNT, Seen, missed};

/// 群 `group` 里 `user` 在平台时刻 `time` 发的、编号是 `id` 的一条（照 NapCat 的 `get_group_msg_history` 交回的样子，不带
/// `post_type` 这几格）。
fn history(group: i64, user: i64, id: i64, time: i64) -> Value {
    json!({
        "group_id": group, "user_id": user, "message_id": id, "time": time,
        "sender": {"user_id": user, "nickname": "小林", "card": ""},
        "message": [{"type": "text", "data": {"text": format!("第 {id} 条")}}],
    })
}

#[test]
fn only_what_came_after_the_last_seen_and_not_her_own_are_backfilled() {
    let messages = json!([
        history(777, 2, 13, 1300),
        history(777, 2, 10, 1000),
        history(777, 1, 12, 1200),
        history(777, 2, 11, 1100),
        history(888, 2, 14, 1400),
    ]);
    let (found, full) = missed(&messages, (Id::from(1), Id::from(777)), 1100);
    let ids: Vec<i64> = found.iter().map(|posted| posted.message_id).collect();
    assert_eq!(
        ids,
        [11, 13],
        "同一秒的也交（核心照命令编号去重），她自己的、别的群的、更早的不交"
    );
    assert!(!full);
    assert_eq!(found[1].bot, Id::from(1));
    assert_eq!(found[1].text, "第 13 条");
}

#[test]
fn a_full_page_newer_than_the_mark_says_more_may_be_missing() {
    let page: Vec<Value> = (0..COUNT as i64)
        .map(|n| history(777, 2, n, 2000 + n))
        .collect();
    let (found, full) = missed(&json!(page), (Id::from(1), Id::from(777)), 1000);
    assert_eq!((found.len(), full), (COUNT, true));
    let (_, full) = missed(&json!(page), (Id::from(1), Id::from(777)), 2000);
    assert!(!full, "最早的一条就是记着的那一秒：前面没漏");
    assert_eq!(
        missed(&json!(null), (Id::from(1), Id::from(777)), 0)
            .0
            .len(),
        0
    );
}

#[test]
fn marks_only_move_forward_and_survive_a_reload() {
    let dir = std::env::temp_dir().join(format!("miyu-im-backfill-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("删得了");
    }
    let mut seen = Seen::load(&dir, Id::from(1));
    assert!(seen.groups().is_empty());
    seen.note(Id::from(777), 1000);
    seen.note(Id::from(777), 900);
    seen.note(Id::from(888), 0);
    seen.flush();
    assert_eq!(
        Seen::load(&dir, Id::from(1)).groups(),
        [(Id::from(777), 1000)]
    );
    assert!(
        Seen::load(&dir, Id::from(2)).groups().is_empty(),
        "号各记各的"
    );
    std::fs::remove_dir_all(&dir).expect("删得掉");
}
