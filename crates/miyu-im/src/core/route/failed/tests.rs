//! 没回成的一轮：出错结束的交回一笔、带上那次出错；走完的不交；人打断的交回但不算要提醒；没回合编号的不看；原话截短、去换行。

use serde_json::{Value, json};

use super::{Failed, Failures, MESSAGE_CHARS};

fn called_error(turn: u64, class: &str, message: &str) -> Value {
    json!({"seq": 10, "kind": "model.called", "turn": turn,
        "body": {"result": "error", "error": {"class": class, "message": message}}})
}

fn ended(turn: u64, reason: &str) -> Value {
    json!({"seq": 11, "kind": "turn.ended", "turn": turn, "body": {"reason": reason}})
}

#[test]
fn a_turn_that_ends_in_error_carries_the_model_error() {
    let mut failures = Failures::default();
    let message =
        "the request would not fit: 113586 tokens used + 4096 reserved for output > window 100000";
    assert_eq!(
        failures.heard("s", &called_error(7, "compaction_paused", message)),
        None
    );
    let failed = failures
        .heard("s", &ended(7, "error"))
        .expect("出错结束的要交回");
    assert_eq!(
        failed,
        Failed {
            turn: 7,
            reason: "error".into(),
            class: "compaction_paused".into(),
            message: message.into(),
        }
    );
    assert!(failed.alarming());
    // 交过就清了：同一轮再推一次结束（照说不会）不带旧的出错。
    let again = failures
        .heard("s", &ended(7, "error"))
        .expect("仍是出错结束");
    assert_eq!(again.class, "");
}

#[test]
fn a_completed_turn_is_quiet_even_after_a_retried_error() {
    let mut failures = Failures::default();
    failures.heard("s", &called_error(3, "rate_limited", "429"));
    assert_eq!(failures.heard("s", &ended(3, "completed")), None);
    // 记下的那次出错跟着清了，不串到下一轮。
    let next = failures
        .heard("s", &ended(4, "step_limit"))
        .expect("步数上限也交回");
    assert_eq!(
        (next.reason.as_str(), next.class.as_str()),
        ("step_limit", "")
    );
    assert!(next.alarming());
}

#[test]
fn errors_stay_with_their_own_session_and_turn() {
    let mut failures = Failures::default();
    failures.heard("a", &called_error(1, "compaction_paused", "x"));
    let other = failures.heard("b", &ended(1, "error")).expect("出错结束");
    assert_eq!(other.class, "");
    let own = failures.heard("a", &ended(1, "error")).expect("出错结束");
    assert_eq!(own.class, "compaction_paused");
}

#[test]
fn interrupted_and_restarted_turns_are_not_alarming() {
    let mut failures = Failures::default();
    for reason in ["interrupted", "restarted"] {
        let failed = failures
            .heard("s", &ended(2, reason))
            .expect("不是走完的都交回");
        assert!(!failed.alarming(), "{reason}");
    }
}

#[test]
fn events_without_a_turn_and_other_kinds_are_ignored() {
    let mut failures = Failures::default();
    let no_turn = json!({"seq": 1, "kind": "turn.ended", "body": {"reason": "error"}});
    assert_eq!(failures.heard("s", &no_turn), None);
    let ok_call = json!({"seq": 2, "kind": "model.called", "turn": 5, "body": {"result": "ok"}});
    assert_eq!(failures.heard("s", &ok_call), None);
    assert_eq!(
        failures.heard("s", &ended(5, "error")).map(|f| f.class),
        Some(String::new())
    );
}

#[test]
fn long_messages_are_cut_to_one_line() {
    let mut failures = Failures::default();
    let long = format!("first\nsecond {}", "长".repeat(MESSAGE_CHARS * 2));
    failures.heard("s", &called_error(9, "bad_request", &long));
    let failed = failures.heard("s", &ended(9, "error")).expect("出错结束");
    assert!(!failed.message.contains('\n'));
    assert!(failed.message.starts_with("first second "));
    assert_eq!(failed.message.chars().count(), MESSAGE_CHARS + 1);
    assert!(failed.message.ends_with('…'));
}
