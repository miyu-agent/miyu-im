//! 戳一戳的编号（施工 I-248，AUD-3 改）。

use super::{is_poke, poke_number};

#[test]
fn poke_numbers_never_look_like_message_ids_and_never_repeat() {
    // 审计 #107：64 位编号的实现（正的）、NapCat 的 32 位编号（正负都有可能）都不是戳一戳；同一毫秒编几个也不重。
    for message in [
        "7438291049281920123",
        "2147483648",
        "2147483647",
        "1",
        "-5",
        "-2147483648",
        "abc",
    ] {
        assert!(!is_poke(message), "{message}");
    }
    let numbers: Vec<i64> = (0..1000).map(|_| poke_number()).collect();
    for number in &numbers {
        assert!(is_poke(&number.to_string()), "{number}");
    }
    let mut unique = numbers.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), numbers.len(), "一个进程里不重");
}
