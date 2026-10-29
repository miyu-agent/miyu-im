//! 认刷屏（施工 Q-5）。

use super::*;

const WINDOW: Duration = Duration::from_secs(60);

#[test]
fn three_of_the_same_from_one_person_in_the_window_is_a_flood() {
    let mut streaks = Streaks::default();
    let start = Instant::now();
    let at = |seconds: u64| start + Duration::from_secs(seconds);
    assert_eq!(
        streaks.seen((Id::from(31), Id::from(7)), ("早安", 1), at(0), WINDOW),
        None
    );
    // 别人插话不打断。
    assert_eq!(
        streaks.seen((Id::from(31), Id::from(8)), ("你好", 2), at(1), WINDOW),
        None
    );
    assert_eq!(
        streaks.seen((Id::from(31), Id::from(7)), (" 早安 ", 3), at(2), WINDOW),
        None,
        "照去掉空白比"
    );
    assert_eq!(
        streaks.seen((Id::from(31), Id::from(7)), ("早安", 4), at(3), WINDOW),
        Some(Flood {
            recall: vec![3, 4],
            fresh: true
        }),
        "头一条留着"
    );
    assert_eq!(
        streaks.seen((Id::from(31), Id::from(7)), ("早安", 5), at(4), WINDOW),
        Some(Flood {
            recall: vec![5],
            fresh: false
        }),
        "再往后的只撤回"
    );
    // 另一个群各算各的。
    assert_eq!(
        streaks.seen((Id::from(32), Id::from(7)), ("早安", 6), at(5), WINDOW),
        None
    );
}

#[test]
fn other_words_wordless_messages_and_the_window_break_a_streak() {
    let mut streaks = Streaks::default();
    let start = Instant::now();
    let at = |seconds: u64| start + Duration::from_secs(seconds);
    streaks.seen((Id::from(31), Id::from(7)), ("a", 1), at(0), WINDOW);
    streaks.seen((Id::from(31), Id::from(7)), ("a", 2), at(1), WINDOW);
    assert_eq!(
        streaks.seen((Id::from(31), Id::from(7)), ("b", 3), at(2), WINDOW),
        None,
        "换了话从头数"
    );
    streaks.seen((Id::from(31), Id::from(7)), ("b", 4), at(3), WINDOW);
    assert_eq!(
        streaks.seen((Id::from(31), Id::from(7)), ("", 5), at(4), WINDOW),
        None,
        "没有字的打断"
    );
    assert_eq!(
        streaks.seen((Id::from(31), Id::from(7)), ("b", 6), at(5), WINDOW),
        None
    );
    streaks.seen((Id::from(31), Id::from(7)), ("b", 7), at(6), WINDOW);
    assert_eq!(
        streaks.seen((Id::from(31), Id::from(7)), ("b", 8), at(70), WINDOW),
        None,
        "头一条到这一条超了时间窗：从这一条重新数"
    );
    streaks.seen((Id::from(31), Id::from(7)), ("b", 9), at(71), WINDOW);
    assert_eq!(
        streaks.seen((Id::from(31), Id::from(7)), ("b", 10), at(72), WINDOW),
        Some(Flood {
            recall: vec![9, 10],
            fresh: true
        })
    );
}
