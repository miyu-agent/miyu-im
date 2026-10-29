//! 改群名片的参数（施工 I-36）。

use serde_json::json;

use super::card;
use miyu_im::chat::Id;

#[test]
fn the_card_is_trimmed_and_the_user_is_optional() {
    assert_eq!(
        card(&json!({"card": " 米尤 "})),
        Ok((None, "米尤".to_string()))
    );
    assert_eq!(
        card(&json!({"user": "40001", "card": "小明"})),
        Ok((Some(Id::from(40001)), "小明".to_string())),
        "号写成字的也认"
    );
    assert_eq!(
        card(&json!({"card": ""})),
        Ok((None, String::new())),
        "空的是清掉"
    );
}

#[test]
fn wrong_arguments_say_which() {
    assert_eq!(card(&json!({})), Err("card must be text"));
    assert_eq!(
        card(&json!({"user": "小明", "card": "x"})),
        Err("user must be a QQ number")
    );
    assert_eq!(
        card(&json!({"card": "长".repeat(61)})),
        Err("card must be at most 60 characters")
    );
}
