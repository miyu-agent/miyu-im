//! 答的几句怎么换字段。

use super::*;

#[test]
fn fields_are_filled_once_and_newlines_become_spaces() {
    assert_eq!(say("none", &[]), "No sponsorship records.");
    assert_eq!(
        say(
            "not-done",
            &[("detail", "amount must be\nabove 0 {detail}")]
        ),
        "Not done: amount must be above 0 {detail}."
    );
    assert_eq!(
        say(
            "recorded",
            &[
                ("id", "3"),
                ("who", "小明 (qq:1)"),
                ("amount", "50 CNY"),
                ("converted", "")
            ]
        ),
        "Recorded #3: 小明 (qq:1) gave 50 CNY."
    );
}

#[test]
fn every_answer_and_the_tool_are_there() {
    for (name, template) in ANSWERS {
        assert!(!template.trim().is_empty(), "{name}");
    }
    let tool: serde_json::Value = serde_json::from_str(TOOL).expect("工具说明是 JSON");
    assert!(tool["description"].is_string());
    assert_eq!(tool["parameters"]["type"], "object");
}
