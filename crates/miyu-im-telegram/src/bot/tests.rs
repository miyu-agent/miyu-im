use serde_json::json;

use super::{State, full_name, read_offset, write_offset};
use miyu_im::chat::Id;

#[test]
fn names_are_first_and_last() {
    assert_eq!(
        full_name(&json!({"first_name": "Alex", "last_name": "Chen"})),
        "Alex Chen"
    );
    assert_eq!(full_name(&json!({"first_name": "Yuki"})), "Yuki");
    assert_eq!(
        full_name(&json!({"last_name": "Li", "first_name": " "})),
        "Li"
    );
    assert_eq!(full_name(&json!({"username": "x"})), "");
}

#[test]
fn the_state_reports_what_is_known() {
    let mut state = State {
        index: 1,
        token: true,
        proxy: "direct".to_string(),
        ..State::default()
    };
    assert_eq!(
        state.fields(),
        json!({"index": 1, "token": "set", "proxy": "direct", "connected": false})
    );
    state.bot = Some((
        Id::new("7890").expect("号"),
        "miyu_bot".to_string(),
        "Miyu".to_string(),
    ));
    state.privacy = Some(true);
    state.connected = true;
    state.trouble = Some("elsewhere");
    assert_eq!(
        state.fields(),
        json!({"index": 1, "token": "set", "proxy": "direct", "connected": true, "bot": "7890", "username": "miyu_bot", "name": "Miyu", "privacy": true, "trouble": "elsewhere"})
    );
}

#[tokio::test]
async fn the_offset_survives_a_restart() {
    let dir = std::env::temp_dir().join(format!("miyu-im-telegram-offset-{}", std::process::id()));
    let file = dir.join("7890.offset");
    assert_eq!(read_offset(&file).await, 0);
    write_offset(&file, 42).await;
    assert_eq!(read_offset(&file).await, 42);
    write_offset(&file, 43).await;
    assert_eq!(read_offset(&file).await, 43);
    assert!(
        !file.with_extension("offset.tmp").exists(),
        "临时文件改名掉了"
    );
    std::fs::remove_dir_all(&dir).expect("删得掉");
}
