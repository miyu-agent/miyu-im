//! 出厂的数据写错了（施工 O-21，`onebot.md` 第一条「场所规则和出厂数据」）：规则、出厂参数、违规词表、判官的说明、警告、目录不在
//! 都算出厂的错，读不出来、交回每一条问题（从 `rules.rs` 挪来：那个文件到了行数上限，施工 I-122）。

use std::path::PathBuf;

use miyu_chat::Source;
use miyu_config::problem::Code;

use crate::rules::{clean, copied_resources};
use crate::support::*;

#[test]
fn factory_mistakes_are_all_reported() {
    let resources = copied_resources();
    let factory = resources.join("packages/im/data");
    std::fs::write(
        factory.join("venues.d/50-defaults.toml"),
        "[[rule]]\nrate = \"abc\"\n",
    )
    .expect("写得进");
    let defaults = std::fs::read_to_string(factory.join("defaults.toml")).expect("读得了");
    std::fs::write(
        factory.join("defaults.toml"),
        defaults.replace("severity_min = 7 ", "severity_min = 70 "),
    )
    .expect("写得进");
    std::fs::remove_file(factory.join("moderation.txt")).expect("删得了");
    let problems =
        crate::support::shipped::factory_at(&package_in(&resources)).expect_err("读不出来");
    let seen = problems
        .iter()
        .map(|problem| {
            (
                problem.code,
                problem.source,
                problem.file.as_str(),
                problem.key.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        seen,
        [
            (
                Code::BadFormat,
                Source::Factory,
                "50-defaults.toml",
                Some("rate")
            ),
            (
                Code::OutOfRange,
                Source::Factory,
                "defaults.toml",
                Some("chatty.severity_min")
            ),
            (Code::Unreadable, Source::Factory, "moderation.txt", None),
        ]
    );
    clean(resources.parent().expect("有上一级"));
}

#[test]
fn broken_judge_texts_are_factory_mistakes() {
    // 判官的说明（施工 O-23 下）：`violations.txt` 的模板写坏了、要了别的字段，记一条 `bad_format`；少了一份记一条读不成。
    let resources = copied_resources();
    let judge = resources.join("packages/im/data/judge");
    std::fs::write(judge.join("violations.txt"), "Severity {nope}.\n").expect("写得进");
    let problems =
        crate::support::shipped::factory_at(&package_in(&resources)).expect_err("模板写坏了");
    assert_eq!(
        problems
            .iter()
            .map(|problem| (problem.code, problem.source, problem.file.as_str()))
            .collect::<Vec<_>>(),
        [(Code::BadFormat, Source::Factory, "judge/violations.txt")]
    );
    assert!(
        problems[0]
            .why
            .as_deref()
            .is_some_and(|why| why.contains("nope")),
        "{problems:?}"
    );
    std::fs::copy(
        resources_file("packages/im/data/judge/violations.txt"),
        judge.join("violations.txt"),
    )
    .expect("抄得了");
    std::fs::remove_file(judge.join("answer.txt")).expect("删得了");
    let problems =
        crate::support::shipped::factory_at(&package_in(&resources)).expect_err("少了一份");
    assert_eq!(
        problems
            .iter()
            .map(|problem| (problem.code, problem.file.as_str()))
            .collect::<Vec<_>>(),
        [(Code::Unreadable, "judge/answer.txt")]
    );
    clean(resources.parent().expect("有上一级"));
}

/// 源码树里资源目录的一份文件。
fn resources_file(path: &str) -> PathBuf {
    resources().join(path)
}

#[test]
fn a_factory_warning_or_a_missing_directory_is_a_mistake_too() {
    let resources = copied_resources();
    let rules = resources.join("packages/im/data/venues.d");
    std::fs::write(rules.join("60-extra.toml"), "nonsense = 1\n").expect("写得进");
    let problems =
        crate::support::shipped::factory_at(&package_in(&resources)).expect_err("警告也算");
    assert_eq!(problems.len(), 1);
    assert_eq!(
        (problems[0].code, problems[0].file.as_str()),
        (Code::UnknownKey, "60-extra.toml")
    );
    std::fs::remove_dir_all(&rules).expect("删得了");
    let problems =
        crate::support::shipped::factory_at(&package_in(&resources)).expect_err("出厂的规则不在");
    assert_eq!(
        problems
            .iter()
            .map(|problem| (problem.code, problem.file.as_str()))
            .collect::<Vec<_>>(),
        [(Code::Unreadable, "venues.d")]
    );
    clean(resources.parent().expect("有上一级"));
}
