//! 违规词表（施工 O-21，`onebot.md` 第一条「场所规则和出厂数据」）：系统那一份整份替换出厂的，读不成的照空的用、报一条；它的问题
//! 排在规则文件的后面。O-58 上从 `rules.rs` 挪来（那个文件过了 500 行）。

use miyu_chat::Source;
use miyu_config::problem::Code;
use miyu_onebot::rules::{Cards, Configured, load};

use crate::rules::{FACTORY_WORDS, clean, system_rule, system_words};
use crate::support::*;

#[test]
fn the_system_word_list_replaces_the_factory_one() {
    let (dir, root) = temp_root();
    let factory = factory();
    let path = system_words(&root, "# 我们群的\nfoo\n\n  bar  \nfoo\n");
    let loaded = load(&factory, &root, &Configured::default(), &Cards::default());
    assert_eq!(
        loaded.keywords,
        ["foo", "bar"],
        "整份替换，照群聊内核的读法"
    );
    assert_eq!(loaded.problems, []);
    std::fs::write(&path, b"foo\n\xff\n").expect("写得进");
    let loaded = load(&factory, &root, &Configured::default(), &Cards::default());
    assert_eq!(loaded.keywords, Vec::<String>::new(), "读不成的照空的用");
    assert_eq!(loaded.problems.len(), 1);
    let problem = &loaded.problems[0];
    assert_eq!(
        (problem.code, problem.source, problem.file.as_str()),
        (Code::NotUtf8, Source::System, "moderation.txt")
    );
    std::fs::remove_file(&path).expect("删得了");
    assert_eq!(
        load(&factory, &root, &Configured::default(), &Cards::default())
            .keywords
            .len(),
        FACTORY_WORDS
    );
    clean(&dir);
}

#[test]
fn word_list_problems_come_after_the_rule_files() {
    let (dir, root) = temp_root();
    system_words(&root, b"\xff");
    system_rule(&root, "99-bad.toml", "nonsense = 1\n");
    let files: Vec<String> = load(&factory(), &root, &Configured::default(), &Cards::default())
        .problems
        .into_iter()
        .map(|problem| problem.file)
        .collect();
    assert_eq!(files, ["99-bad.toml", "moderation.txt"]);
    clean(&dir);
}
