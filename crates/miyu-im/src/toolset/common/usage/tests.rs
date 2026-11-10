//! 查用量（施工 I-206）的纯逻辑：读参数、数和金额的写法。

use serde_json::json;

use super::{money, priced, short, wanted};

const BAD: &str = "user must be a QQ number or a name";

#[test]
fn hours_default_to_a_day_and_are_bounded() {
    assert_eq!(wanted(&json!({}), BAD), Ok((24, None)));
    assert_eq!(
        wanted(&json!({"hours": 168, "user": 10002}), BAD),
        Ok((168, Some("10002".to_string())))
    );
    assert_eq!(
        wanted(&json!({"user": " 小林 "}), BAD),
        Ok((24, Some("小林".to_string())))
    );
    for bad in [json!(0), json!(721), json!("一天")] {
        assert_eq!(
            wanted(&json!({ "hours": bad }), BAD),
            Err("hours must be 1 to 720")
        );
    }
    assert_eq!(wanted(&json!({"user": true}), BAD), Err(BAD));
    assert_eq!(wanted(&json!({"user": "  "}), BAD), Err(BAD));
}

#[test]
fn numbers_and_money_read_like_a_person_wrote_them() {
    assert_eq!(short(987), "987");
    assert_eq!(short(34_500), "34.5k");
    assert_eq!(short(1_234_567), "1.2M");
    let converted =
        json!({"currency": "CNY", "amount": 1.234, "rest": [{"currency": "USD", "amount": 0.1}]});
    assert_eq!(money(&converted).as_deref(), Some("1.23 CNY + 0.10 USD"));
    // 不到 1 的多留两位、多出来的 0 不写（施工 I-209）：不到一分钱的不写成 0.00。
    let small = |amount: f64| money(&json!({"currency": "USD", "amount": amount})).expect("有");
    assert_eq!(small(0.0832), "0.0832 USD");
    assert_eq!(small(0.004), "0.004 USD");
    assert_eq!(small(0.5), "0.50 USD");
    assert_eq!(small(0.00001), "<0.0001 USD");
    assert_eq!(small(0.0), "0.00 USD");
    assert_eq!(small(12.3456), "12.35 USD");
    assert_eq!(money(&json!(null)), None);
    assert_eq!(money(&json!({"rest": []})), None);
    assert_eq!(priced(Some("1.23 CNY".into()), 0), "1.23 CNY");
    assert_eq!(
        priced(Some("1.23 CNY".into()), 2),
        "1.23 CNY (2 more request(s) have no recorded cost)"
    );
    assert_eq!(priced(None, 0), "no cost recorded");
    assert_eq!(priced(None, 3), "no cost recorded for 3 request(s)");
}
