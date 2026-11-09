use serde_json::json;

use miyu_im::chat::MenuItem;

use super::build;

fn item(name: &str, summary: &str) -> MenuItem {
    MenuItem {
        name: name.to_string(),
        summary: summary.to_string(),
    }
}

#[test]
fn names_are_rewritten_for_telegram_and_mapped_back() {
    let (commands, names) = build(&[
        item("clear-memory", "清掉记忆"),
        item("help", ""),
        item("github:review", "MCP 的提示词"),
        item("技能", "不是英文名的技能"),
        item(&"x".repeat(33), "太长"),
    ]);
    assert_eq!(
        commands,
        [
            json!({"command": "clear_memory", "description": "清掉记忆"}),
            json!({"command": "help", "description": "help"}),
        ]
    );
    assert_eq!(
        names.get("clear_memory").map(String::as_str),
        Some("clear-memory")
    );
    assert_eq!(names.len(), 2);
}

#[test]
fn at_most_a_hundred_and_long_summaries_are_cut() {
    let many: Vec<MenuItem> = (0..120)
        .map(|n| item(&format!("c{n}"), &"说".repeat(300)))
        .collect();
    let (commands, _) = build(&many);
    assert_eq!(commands.len(), 100);
    assert_eq!(
        commands[0]["description"]
            .as_str()
            .map(|text| text.chars().count()),
        Some(256)
    );
}
