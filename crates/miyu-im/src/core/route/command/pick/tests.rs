//! `/model`、`/persona`、`/preset`（施工 I-53、I-57、I-191、I-194）的纯逻辑：认不认这条命令、后面的词说的是哪一种、确切的名字、
//! 搜、核心列的里头能选哪些。

use serde_json::json;

use super::{Asked, Choice, Kind, asked, choices, exact, picked, search};

fn models(names: &[&str]) -> Vec<Choice> {
    names
        .iter()
        .map(|name| Choice {
            id: (*name).to_string(),
            name: (*name).to_string(),
        })
        .collect()
}

fn names() -> Vec<Choice> {
    models(&[
        "@lite",
        "dev/antigravity/gemini-3.8-flash",
        "dev/antigravity/gemini-3.8-flash-tiered",
        "dev/clinepass/cline-pass/deepseek-v4.1-flash",
        "dev/opencode-zen/gemini-3.8-flash",
    ])
}

fn personas() -> Vec<Choice> {
    [
        ("default", "Miyu"),
        ("engineer", "软件工程师"),
        ("cat", "猫娘"),
    ]
    .map(|(id, name)| Choice {
        id: id.to_string(),
        name: name.to_string(),
    })
    .to_vec()
}

fn id(found: Option<&Choice>) -> Option<&str> {
    found.map(|choice| choice.id.as_str())
}

#[test]
fn only_the_three_commands_and_their_words_are_taken() {
    assert_eq!(picked("/model"), Some((Kind::Model, "")));
    assert_eq!(
        picked("  /model  deepseek  "),
        Some((Kind::Model, "deepseek"))
    );
    assert_eq!(picked("/persona"), Some((Kind::Persona, "")));
    assert_eq!(picked("/persona 2"), Some((Kind::Persona, "2")));
    assert_eq!(
        picked("/preset search 无"),
        Some((Kind::Preset, "search 无"))
    );
    assert_eq!(picked("/models"), None, "别的命令不算");
    assert_eq!(picked("/personas"), None);
    assert_eq!(picked("/clear"), None);
    assert_eq!(picked("看看 /model"), None);
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
        id(exact(
            Kind::Model,
            "dev/antigravity/gemini-3.8-flash",
            &names
        )),
        Some("dev/antigravity/gemini-3.8-flash")
    );
    assert_eq!(
        id(exact(Kind::Model, "@LITE", &names)),
        Some("@lite"),
        "不分大小写"
    );
    assert_eq!(
        id(exact(Kind::Model, "deepseek-v4.1-flash", &names)),
        Some("dev/clinepass/cline-pass/deepseek-v4.1-flash"),
        "只写模型那一段、只有一个对得上的也算"
    );
    assert_eq!(
        exact(Kind::Model, "gemini-3.8-flash", &names),
        None,
        "两个供应商都有的不算"
    );
    assert_eq!(exact(Kind::Model, "deepseek", &names), None, "模糊的不换");
}

#[test]
fn a_persona_switches_by_number_id_or_name() {
    let personas = personas();
    assert_eq!(id(exact(Kind::Persona, "2", &personas)), Some("engineer"));
    assert_eq!(
        id(exact(Kind::Persona, "Engineer", &personas)),
        Some("engineer")
    );
    assert_eq!(id(exact(Kind::Persona, "猫娘", &personas)), Some("cat"));
    assert_eq!(id(exact(Kind::Persona, "miyu", &personas)), Some("default"));
    assert_eq!(exact(Kind::Persona, "猫", &personas), None, "模糊的不换");
    let twins = [("a", "同名"), ("b", "同名")]
        .map(|(id, name)| Choice {
            id: id.to_string(),
            name: name.to_string(),
        })
        .to_vec();
    assert_eq!(
        exact(Kind::Preset, "同名", &twins),
        None,
        "两个叫一样的不猜"
    );
    assert_eq!(personas[1].shown(), "软件工程师 (engineer)");
    assert_eq!(names()[0].shown(), "@lite", "名字和编号一样的只写一遍");
}

#[test]
fn search_lists_every_match_with_its_number_in_the_whole_list() {
    let names = names();
    let found = search("zen flash", &names);
    assert_eq!(
        found,
        [(5, &names[4])],
        "几个词先后不论，编号是它在全部里的第几个"
    );
    assert_eq!(search("gemini flash", &names).len(), 3);
    let all = search("", &names);
    let numbered: Vec<(usize, &Choice)> = names
        .iter()
        .enumerate()
        .map(|(at, name)| (at + 1, name))
        .collect();
    assert_eq!(all, numbered, "没写关键词的照原来的先后全列、从 1 编号");
    assert!(search("claude", &names).is_empty());
    let many: Vec<String> = (0..60).map(|n| format!("p/m{n}")).collect();
    let many: Vec<&str> = many.iter().map(String::as_str).collect();
    assert_eq!(search("m", &models(&many)).len(), 60, "不再只列 10 个");
    let personas = personas();
    assert_eq!(search("工程师", &personas), [(2, &personas[1])], "名字也搜");
    assert_eq!(search("cat", &personas), [(3, &personas[2])], "编号也搜");
}

#[test]
fn search_ignores_dots_dashes_and_case() {
    let names = models(&[
        "magpie/anthropic/claude-opus-4-5",
        "magpie/anthropic/claude-opus-5-5",
    ]);
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
    assert_eq!(id(exact(Kind::Model, "1", &names)), Some("@lite"));
    assert_eq!(
        id(exact(Kind::Model, "4", &names)),
        Some("dev/clinepass/cline-pass/deepseek-v4.1-flash")
    );
    assert_eq!(exact(Kind::Model, "0", &names), None, "从 1 编号");
    assert_eq!(exact(Kind::Model, "6", &names), None, "超出的不换");
}

#[test]
fn the_choices_come_from_what_the_core_lists() {
    let listed = json!({
        "pools": [{"name": "lite"}],
        "providers": [{"models": [{"ref": "p/a"}, {"ref": "p/b"}]}],
    });
    let got: Vec<String> = choices(Kind::Model, &listed)
        .into_iter()
        .map(|choice| choice.id)
        .collect();
    assert_eq!(got, ["@lite", "p/a", "p/b"], "池在前");
    let listed = json!({"personas": [
        {"persona": "default", "name": "Miyu"},
        {"persona": "broken", "problem": "写错了"},
    ]});
    assert_eq!(
        choices(Kind::Persona, &listed),
        [Choice {
            id: "default".to_string(),
            name: "Miyu".to_string()
        }],
        "写错了的不列"
    );
    let listed = json!({"presets": [{"preset": "none", "name": "无"}]});
    assert_eq!(choices(Kind::Preset, &listed)[0].shown(), "无 (none)");
}
