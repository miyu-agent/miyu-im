//! `/model`（施工 I-53、I-57）的纯逻辑：认不认这条命令、后面的词说的是哪一种、确切的名字、搜。

use super::{Asked, asked, exact, model_query, search};

fn names() -> Vec<String> {
    [
        "@lite",
        "dev/antigravity/gemini-3.8-flash",
        "dev/antigravity/gemini-3.8-flash-tiered",
        "dev/clinepass/cline-pass/deepseek-v4.1-flash",
        "dev/opencode-zen/gemini-3.8-flash",
    ]
    .map(str::to_string)
    .to_vec()
}

#[test]
fn only_slash_model_and_its_words_are_taken() {
    assert_eq!(model_query("/model"), Some(""));
    assert_eq!(model_query("  /model  deepseek  "), Some("deepseek"));
    assert_eq!(model_query("/models"), None, "别的命令不算");
    assert_eq!(model_query("/clear"), None);
    assert_eq!(model_query("看看 /model"), None);
}

#[test]
fn the_words_say_now_search_clear_or_set() {
    assert_eq!(asked(""), Asked::Now);
    assert_eq!(
        asked("search deepseek flash"),
        Asked::Search("deepseek flash")
    );
    assert_eq!(asked("SEARCH  zen"), Asked::Search("zen"));
    assert_eq!(asked("search"), Asked::Search(""), "没写关键词的照先后列");
    assert_eq!(asked("默认"), Asked::Clear);
    assert_eq!(asked("Default"), Asked::Clear);
    assert_eq!(asked("@lite"), Asked::Set("@lite"));
    assert_eq!(
        asked("searching"),
        Asked::Set("searching"),
        "整个词才算 search"
    );
}

#[test]
fn only_an_exact_name_switches() {
    let names = names();
    assert_eq!(
        exact("dev/antigravity/gemini-3.8-flash", &names).as_deref(),
        Some("dev/antigravity/gemini-3.8-flash")
    );
    assert_eq!(
        exact("@LITE", &names).as_deref(),
        Some("@lite"),
        "不分大小写"
    );
    assert_eq!(
        exact("deepseek-v4.1-flash", &names).as_deref(),
        Some("dev/clinepass/cline-pass/deepseek-v4.1-flash"),
        "只写模型那一段、只有一个对得上的也算"
    );
    assert_eq!(
        exact("gemini-3.8-flash", &names),
        None,
        "两个供应商都有的不算"
    );
    assert_eq!(exact("deepseek", &names), None, "模糊的不换");
}

#[test]
fn search_lists_every_match_with_its_number_in_the_whole_list() {
    let names = names();
    let found = search("zen flash", &names);
    assert_eq!(
        found,
        [(5, "dev/opencode-zen/gemini-3.8-flash".to_string())],
        "几个词先后不论，编号是它在全部里的第几个"
    );
    assert_eq!(search("gemini flash", &names).len(), 3);
    let all = search("", &names);
    let numbered: Vec<(usize, String)> = names
        .iter()
        .cloned()
        .enumerate()
        .map(|(at, name)| (at + 1, name))
        .collect();
    assert_eq!(all, numbered, "没写关键词的照原来的先后全列、从 1 编号");
    assert!(search("claude", &names).is_empty());
    let many: Vec<String> = (0..60).map(|n| format!("p/m{n}")).collect();
    assert_eq!(search("m", &many).len(), 60, "不再只列 10 个");
}

#[test]
fn search_ignores_dots_dashes_and_case() {
    let names = [
        "magpie/anthropic/claude-opus-4-5",
        "magpie/anthropic/claude-opus-5-5",
    ]
    .map(str::to_string)
    .to_vec();
    for query in ["opus5.5", "OPUS 5-5", "opus55", "claude_opus_5_5"] {
        let found = search(query, &names);
        assert_eq!(
            found.first().map(|(number, _)| *number),
            Some(2),
            "{query}: {found:?}"
        );
    }
}

#[test]
fn a_number_from_the_list_switches_too() {
    let names = names();
    assert_eq!(exact("1", &names).as_deref(), Some("@lite"));
    assert_eq!(
        exact("4", &names).as_deref(),
        Some("dev/clinepass/cline-pass/deepseek-v4.1-flash")
    );
    assert_eq!(exact("0", &names), None, "从 1 编号");
    assert_eq!(exact("6", &names), None, "超出的不换");
}
