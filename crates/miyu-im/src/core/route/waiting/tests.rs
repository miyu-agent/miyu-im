use super::{MOST, Waiting};
use crate::core::route::judges::Tag;

fn tag(number: i64) -> Tag {
    Tag {
        id: format!("qq:1:{number}:0"),
        number,
        venue: "qq:group:7".to_string(),
        who: "小林".to_string(),
        text: "在吗".to_string(),
        facts: Vec::new(),
    }
}

#[test]
fn rounds_wait_in_order_up_to_the_limit() {
    let mut waiting = Waiting::default();
    for number in 0..i64::try_from(MOST).expect("小") {
        assert!(waiting.push("s", &[1], &tag(number)));
    }
    assert!(!waiting.push("s", &[2], &tag(99)), "满了不排");
    assert!(waiting.push("other", &[3], &tag(100)), "各群各排");
    assert_eq!(waiting.pop("s").map(|(_, tag)| tag.number), Some(0));
    assert_eq!(waiting.pop("s").map(|(_, tag)| tag.number), Some(1));
    assert!(waiting.pop("nobody").is_none());
}
