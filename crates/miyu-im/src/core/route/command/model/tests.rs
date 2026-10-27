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
fn search_lists_the_closest_first_and_counts_the_rest() {
    let names = names();
    let (found, more) = search("zen flash", &names);
    assert_eq!(
        found,
        ["dev/opencode-zen/gemini-3.8-flash"],
        "几个词先后不论"
    );
    assert_eq!(more, 0);
    let (found, _) = search("gemini flash", &names);
    assert_eq!(found.len(), 3, "{found:?}");
    let (found, more) = search("", &names);
    assert_eq!(found, names, "没写关键词的照原来的先后");
    assert_eq!(more, 0);
    assert!(search("claude", &names).0.is_empty());
    let many: Vec<String> = (0..13).map(|n| format!("p/m{n}")).collect();
    let (found, more) = search("m", &many);
    assert_eq!((found.len(), more), (10, 3), "最多列 10 个，多的说还有几个");
}
