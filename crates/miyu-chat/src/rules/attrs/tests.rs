use miyu_config::Value;
use miyu_config::problem::Code;

use crate::rules::Rules;
use crate::rules::test_support::{group, system};

#[test]
fn recent_counts_from_one_to_a_thousand() {
    // 施工 O-62：群聊近况带多少条，照核心 `venue.session` 的 `recent` 收 1 到 1000；出了范围、写错类型的那一项丢掉、记问题。
    for (count, kept, problem) in [
        ("1", Some(1), None),
        ("1000", Some(1000), None),
        ("0", None, Some(Code::OutOfRange)),
        ("1001", None, Some(Code::OutOfRange)),
        ("\"30\"", None, Some(Code::WrongType)),
    ] {
        let read = Rules::parse(&[system("a.toml", &format!("[[rule]]\nrecent = {count}\n"))]);
        let codes: Vec<Code> = read.problems.iter().map(|problem| problem.code).collect();
        assert_eq!(codes, Vec::from_iter(problem), "{count}");
        let got = read
            .rules
            .resolve(&group("1"))
            .entries
            .get("recent")
            .map(|entry| entry.value.clone());
        assert_eq!(got, kept.map(Value::Int), "{count}");
    }
}

#[test]
fn a_model_is_written_as_a_reference() {
    // 施工 O-58 补三：`<供应商>/<模型>` 或 `@<池>`，别的写法那一项丢掉。
    for (written, kept) in [
        ("\"deepseek/v4\"", true),
        ("\"@fast\"", true),
        ("\"v4\"", false),
        ("\"\"", false),
        ("1", false),
    ] {
        let read = Rules::parse(&[system("a.toml", &format!("[[rule]]\nmodel = {written}\n"))]);
        assert_eq!(read.problems.is_empty(), kept, "{written}");
        let got = read
            .rules
            .resolve(&group("1"))
            .entries
            .contains_key("model");
        assert_eq!(got, kept, "{written}");
    }
}
