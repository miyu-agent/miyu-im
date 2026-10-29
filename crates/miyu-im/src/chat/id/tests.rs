use super::*;

#[test]
fn integers_round_trip_as_numbers_and_text_as_text() {
    let qq: Id = serde_json::from_value(serde_json::json!(130515298)).unwrap();
    assert_eq!(
        serde_json::to_value(&qq).unwrap(),
        serde_json::json!(130515298)
    );
    let group: Id = serde_json::from_value(serde_json::json!(-1001987654321_i64)).unwrap();
    assert_eq!(group.as_str(), "-1001987654321");
    assert_eq!(
        serde_json::to_value(&group).unwrap(),
        serde_json::json!(-1001987654321_i64)
    );
    let openid: Id = serde_json::from_value(serde_json::json!("C9F778FE6ADF")).unwrap();
    assert_eq!(
        serde_json::to_value(&openid).unwrap(),
        serde_json::json!("C9F778FE6ADF")
    );
    let typed: Id = serde_json::from_value(serde_json::json!("20017")).unwrap();
    assert_eq!(typed, Id::from(20017));
}

#[test]
fn integers_a_browser_cannot_read_exactly_stay_text() {
    let big = Id::from(i64::MAX);
    assert_eq!(
        serde_json::to_value(&big).unwrap(),
        serde_json::json!(i64::MAX.to_string())
    );
}

#[test]
fn ids_that_would_break_a_venue_are_refused() {
    for bad in ["", "a:b", "a\nb", "a b", &"x".repeat(MAX_LEN + 1)] {
        assert_eq!(Id::new(bad), None, "{bad:?}");
        assert!(serde_json::from_value::<Id>(serde_json::json!(bad)).is_err());
    }
    assert!(Id::new(&"x".repeat(MAX_LEN)).is_some());
    assert!(Id::new("wxid_abc@chatroom").is_some());
}

#[test]
fn only_plain_decimal_text_reads_as_a_number() {
    assert_eq!(Id::from(-5).number(), Some(-5));
    assert_eq!(Id::new("007").unwrap().number(), None);
    assert_eq!(Id::new("+7").unwrap().number(), None);
    assert_eq!(Id::new("abc").unwrap().number(), None);
}

#[test]
fn numbers_sort_by_value_before_text() {
    let mut ids = [
        Id::new("b").unwrap(),
        Id::from(100),
        Id::new("a").unwrap(),
        Id::from(9),
        Id::from(-1),
    ];
    ids.sort();
    let texts: Vec<&str> = ids.iter().map(Id::as_str).collect();
    assert_eq!(texts, ["-1", "9", "100", "a", "b"]);
}

#[test]
fn printing_shows_the_bare_id() {
    assert_eq!(
        format!("{:?} {}", Id::from(42), Id::new("x").unwrap()),
        "42 x"
    );
}
