//! 汇率的回答怎么读。

use super::*;

#[test]
fn only_a_successful_positive_rate_counts() {
    let good = br#"{"result":"success","rates":{"CNY":7.1440,"USD":1}}"#;
    assert_eq!(parsed(good, "CNY"), Some(7.144));
    assert_eq!(parsed(good, "EUR"), None, "没有这一项");
    assert_eq!(
        parsed(br#"{"result":"error","rates":{"CNY":7}}"#, "CNY"),
        None
    );
    assert_eq!(
        parsed(br#"{"result":"success","rates":{"CNY":0}}"#, "CNY"),
        None
    );
    assert_eq!(parsed(b"not json", "CNY"), None);
}
