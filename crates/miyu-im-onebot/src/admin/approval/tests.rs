//! 认加群申请、读 AI 的结论（施工 Q-7）。

use super::*;

#[test]
fn only_join_requests_with_a_flag_are_read() {
    let frame = json!({"post_type": "request", "request_type": "group", "sub_type": "add", "self_id": 1, "group_id": 31, "user_id": 7, "comment": "问题：暗号\n答案：芝麻开门", "flag": "f1"});
    assert_eq!(
        read(&frame),
        Some(Applicant {
            bot: Id::from(1),
            group: Id::from(31),
            user: Id::from(7),
            comment: "问题：暗号\n答案：芝麻开门".to_string(),
            flag: "f1".to_string()
        })
    );
    let mut invite = frame.clone();
    invite["sub_type"] = json!("invite");
    assert_eq!(read(&invite), None, "邀请她进群的不是");
    let mut flagless = frame.clone();
    flagless["flag"] = json!("");
    assert_eq!(read(&flagless), None);
    let mut quiet = frame;
    quiet.as_object_mut().expect("是对象").remove("comment");
    assert_eq!(
        read(&quiet).map(|one| one.comment),
        Some(String::new()),
        "没写的是空字"
    );
}

#[test]
fn the_verdict_is_read_from_the_json_and_the_reason_cut() {
    assert_eq!(
        verdict("好的：\n{\"verdict\": \"approve\", \"reason\": \" 答对了 \"}"),
        Some((Verdict::Approve, "答对了".to_string()))
    );
    assert_eq!(
        verdict(r#"{"verdict": "reject"}"#),
        Some((Verdict::Reject, String::new()))
    );
    let long = format!(r#"{{"verdict": "hold", "reason": "{}"}}"#, "很".repeat(60));
    assert_eq!(
        verdict(&long).map(|(_, reason)| reason.chars().count()),
        Some(REASON)
    );
    for bad in ["通过", r#"{"verdict": "maybe"}"#, "{not json}", ""] {
        assert_eq!(verdict(bad), None, "{bad}");
    }
}

#[test]
fn snowluma_flags_are_fixed_and_filtered_ones_are_known() {
    // 施工 I-17，照旧版 `group_join.rs`：加群申请的标记里事件种类 2 改成 1，别的照原样。
    assert_eq!(action_flag("slreq:1:31:7:2:99"), "slreq:1:31:7:1:99");
    assert_eq!(action_flag("slreq:1:31:7:1:99"), "slreq:1:31:7:1:99");
    assert_eq!(
        action_flag("slreq:2:31:7:2:99"),
        "slreq:2:31:7:2:99",
        "不是这一版"
    );
    assert_eq!(action_flag("1700000000"), "1700000000", "NapCat 的照原样");
    assert!(filtered("slreq:1:31:7:2:1"));
    assert!(!filtered("slreq:1:31:7:2:0"));
    assert!(!filtered("1700000001"));
}

#[test]
fn control_characters_are_taken_out_of_the_reason() {
    assert_eq!(
        verdict("{\"verdict\": \"reject\", \"reason\": \"答\\u0007错\\n了\"}"),
        Some((Verdict::Reject, "答错了".to_string()))
    );
}
