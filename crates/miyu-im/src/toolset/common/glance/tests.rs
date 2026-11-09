//! 定下一次看一眼的参数：整数照收，字写的整数也收，小于 1 的当 1、不设上限；没写、不是整数的报错。

use serde_json::json;

use super::minutes;

#[test]
fn minutes_are_whole_numbers_at_least_one_without_a_ceiling() {
    assert_eq!(minutes(&json!({"minutes": 25})), Ok(25));
    assert_eq!(minutes(&json!({"minutes": "40"})), Ok(40));
    assert_eq!(minutes(&json!({"minutes": 0})), Ok(1));
    assert_eq!(minutes(&json!({"minutes": -5})), Ok(1));
    assert_eq!(minutes(&json!({"minutes": 100_000})), Ok(100_000));
    assert!(minutes(&json!({})).is_err());
    assert!(minutes(&json!({"minutes": "soon"})).is_err());
}
