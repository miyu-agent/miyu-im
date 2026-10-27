//! `/model`（施工 I-53）的纯逻辑：认不认这条命令、照词找模型。

use super::{Found, find, model_query};

fn names() -> Vec<String> {
    [
        "dev/antigravity/gemini-3.8-flash",
        "dev/antigravity/gemini-3.8-flash-tiered",
        "dev/clinepass/cline-pass/deepseek-v4.1-flash",
        "dev/opencode-zen/gemini-3.8-flash",
        "@lite",
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
fn a_model_is_found_exactly_fuzzily_or_listed() {
    let names = names();
    assert_eq!(
        find("dev/antigravity/gemini-3.8-flash", &names),
        Found::One("dev/antigravity/gemini-3.8-flash".to_string()),
        "一模一样的直接是它，哪怕别的也对得上"
    );
    assert_eq!(
        find("deepseek", &names),
        Found::One("dev/clinepass/cline-pass/deepseek-v4.1-flash".to_string())
    );
    assert_eq!(
        find("zen flash", &names),
        Found::One("dev/opencode-zen/gemini-3.8-flash".to_string()),
        "几个词先后不论"
    );
    assert_eq!(
        find("@LITE", &names),
        Found::One("@lite".to_string()),
        "不分大小写"
    );
    match find("gemini flash", &names) {
        Found::Many(listed, more) => {
            assert_eq!(listed.len(), 3, "{listed:?}");
            assert_eq!(more, 0);
        }
        other => panic!("几个对得上的列出来：{other:?}"),
    }
    assert_eq!(find("claude", &names), Found::None);
}
