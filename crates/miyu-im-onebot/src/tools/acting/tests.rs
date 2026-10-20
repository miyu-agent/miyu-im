//! 一次调用没成算成哪一句（施工 O-31，`onebot.md` 第一条「平台工具（一）」第 5 条）：平台回了失败的照驱动读好的原话
//! （`said`，怎么读在 `tests/calls.rs`）；等不到、断了各是一句。

use super::{Undone, undone};
use miyu_im::chat::CallError;

#[test]
fn a_refusal_carries_what_the_platform_said() {
    let failed = CallError::Failed {
        said: "cannot ban owner".to_string(),
        message: "别的".to_string(),
    };
    assert_eq!(
        undone(failed),
        Undone("failed", Some("cannot ban owner".to_string()))
    );
}

#[test]
fn no_answer_and_a_dropped_link_are_their_own_sentences() {
    assert_eq!(undone(CallError::Timeout), Undone("unanswered", None));
    assert_eq!(undone(CallError::Closed), Undone("unreachable", None));
}
