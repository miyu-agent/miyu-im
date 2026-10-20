//! 认加群申请、读 AI 的结论（施工 Q-7）。

use super::*;

#[test]
fn only_join_requests_with_a_flag_are_read() {
    let frame = json!({"post_type": "request", "request_type": "group", "sub_type": "add", "self_id": 1, "group_id": 31, "user_id": 7, "comment": "问题：暗号\n答案：芝麻开门", "flag": "f1"});
    assert_eq!(
        read(&frame),
        Some(Applicant {
            bot: 1,
            group: 31,
            user: 7,
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
