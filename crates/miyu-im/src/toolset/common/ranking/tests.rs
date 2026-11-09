use serde_json::json;

use super::wanted;

#[test]
fn days_and_limit_are_read_with_their_defaults() {
    assert_eq!(wanted(&json!({})), Ok((7, 10)));
    assert_eq!(wanted(&json!({"days": 30, "limit": "5"})), Ok((30, 5)));
    assert_eq!(wanted(&json!({"days": 1})), Ok((1, 10)));
    assert!(wanted(&json!({"days": 3})).is_err());
    assert!(wanted(&json!({"limit": 51})).is_err());
    assert!(wanted(&json!({"limit": 0})).is_err());
}
