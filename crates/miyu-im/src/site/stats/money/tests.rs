//! 金额的算法（施工 I-104）。

use serde_json::json;

use super::*;

#[test]
fn converted_amounts_add_up_and_the_rest_is_kept_apart() {
    let rows = [
        json!({"unpriced": 0, "amounts": [{"currency": "USD", "amount": 0.5}], "converted": {"currency": "USD", "amount": 0.5}}),
        json!({"unpriced": 3, "amounts": [{"currency": "USD", "amount": 0.25}, {"currency": "XYZ", "amount": 7.0}],
               "converted": {"currency": "USD", "amount": 0.25, "rest": [{"currency": "XYZ", "amount": 7.0}]}}),
        // 一笔都没换成的整行照 `amounts`。
        json!({"unpriced": 1, "amounts": [{"currency": "XYZ", "amount": 1.0}]}),
    ];
    let money = Money::of(&rows.iter().collect::<Vec<_>>());
    assert_eq!(
        money.shown(),
        json!({"currency": "USD", "amount": 0.75, "unconverted": [{"currency": "XYZ", "amount": 8.0}], "unpriced": 4})
    );
}

#[test]
fn nothing_converted_has_no_amount() {
    let rows = [json!({"unpriced": 2, "amounts": []})];
    let money = Money::of(&rows.iter().collect::<Vec<_>>());
    assert_eq!(money.amount(), None);
    assert_eq!(money.shown()["unpriced"], 2);
    assert_eq!(Money::default().shown()["amount"], json!(null));
}
