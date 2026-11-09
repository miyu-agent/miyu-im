use serde_json::json;

use miyu_im::chat::Lead;

use super::{parts, split};
use crate::known::{Known, Person};

#[test]
fn long_text_is_split_by_utf16_units_at_line_breaks_first() {
    let text = format!("{}\n{}", "a".repeat(6), "b".repeat(6));
    assert_eq!(split(&text, 10), ["aaaaaa", "bbbbbb"]);
    // emoji 一个占两个单位：照单位算，不切开一个字。
    let emoji = "😀".repeat(6);
    let cut = split(&emoji, 5);
    assert!(
        cut.iter().all(|part| part.encode_utf16().count() <= 5),
        "{cut:?}"
    );
    assert_eq!(cut.concat(), emoji);
    assert_eq!(split("短的", 4096), ["短的"]);
    assert_eq!(split("", 4096), [""]);
    // 没有换行的找空白、标点断。
    assert_eq!(split("一二三。四五六七", 5), ["一二三。", "四五六七"]);
}

#[test]
fn ats_use_usernames_or_text_mentions_and_only_the_first_part_carries_them() {
    let known = Known::default();
    known.set_person(
        "7",
        Person {
            name: "Alex".to_string(),
            username: Some("alex".to_string()),
        },
    );
    known.set_person(
        "8",
        Person {
            name: "雪".to_string(),
            username: None,
        },
    );
    let lead = Lead {
        reply: Some("5".to_string()),
        at: Some("7".to_string()),
        more: vec!["8".to_string(), "7".to_string()],
    };
    let cut = parts("你好", &lead, &known, 4096);
    assert_eq!(cut.len(), 1);
    assert_eq!(cut[0].0, "@alex @雪 你好");
    assert_eq!(
        cut[0].1,
        [json!({"type": "text_mention", "offset": 6, "length": 2, "user": {"id": 8}})]
    );
    // 没见过的人照号写。
    let unknown = Lead {
        at: Some("99".to_string()),
        ..Lead::default()
    };
    assert_eq!(parts("x", &unknown, &known, 4096)[0].0, "@99 x");
    let long = parts(
        &format!("{}\n{}", "a".repeat(8), "b".repeat(8)),
        &lead,
        &known,
        12,
    );
    assert!(long.len() > 1);
    assert!(long[1..].iter().all(|(_, entities)| entities.is_empty()));
}
