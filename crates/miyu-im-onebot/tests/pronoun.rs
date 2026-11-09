//! 给人看的字不写「她」（施工 O-29，`onebot.md` 第一条「给人看的字」；项目主人 2026-10-10 定）：接入通讯平台 的清单（名字、说明、
//! 配置项的字）、im 自己的页面的字（`site/texts.js`，施工 I-180：只看值，键是代码里的名字）、赞助记录的清单（施工 I-180）、桥说给人听的字、出厂参数文件和出厂场所规则里的说明、场所卡片两页的字（施工 O-58 下），都不用「她」「彼女」「she」「her」，写「AI」或者换个说法。
//! 给模型看的字（判官的模板、事实、工具说明）不在这一条里。

/// 出厂的资源目录（拼成的，施工 S-6 中）。
fn resources() -> std::path::PathBuf {
    crate::support::resources()
}

/// 给人看的那几份，相对资源目录。
const HUMAN_FACING: [&str; 8] = [
    "packages/miyu-im/package.toml",
    "packages/miyu-im/site/texts.js",
    "packages/miyu-im/data/human/zh.json",
    "packages/miyu-im/data/human/en.json",
    "packages/miyu-im/data/human/ja.json",
    "packages/miyu-im/data/defaults.toml",
    "packages/miyu-im/data/venues.d/50-defaults.toml",
    "packages/miyu-im/data/cards/human/page.json",
];

/// 一行里有没有用代词说 AI：中文、日文照字，英文照整词（不分大小写）。
fn pronoun(line: &str) -> bool {
    if line.contains('她') || line.contains("彼女") {
        return true;
    }
    line.split(|c: char| !c.is_ascii_alphabetic()).any(|word| {
        matches!(
            word.to_ascii_lowercase().as_str(),
            "she" | "her" | "hers" | "herself"
        )
    })
}

/// 仓库里另外几份给人看的（施工 I-180）：赞助记录的清单、后台页的字不在 im 的资源目录里，照仓库的路径读。
const IN_REPO: [&str; 2] = [
    "miyu-im-sponsor/package.toml",
    "miyu-im-sponsor/page/texts.js",
];

/// 一行里给人看的那一段：页面的字（`texts.js`）只看值，键（`"activity/glance-hers": …`）是代码里的名字。
fn shown<'a>(file: &str, line: &'a str) -> &'a str {
    if file.ends_with("texts.js") {
        return line.split_once("\": ").map_or(line, |(_, value)| value);
    }
    line
}

/// 文件 `file`（叫 `name`）里有代词的那几行。
fn lines_with_pronouns(name: &str, file: &std::path::Path, found: &mut Vec<String>) {
    let text = std::fs::read_to_string(file).expect("出厂的文件读得出来");
    for (number, line) in text.lines().enumerate() {
        if pronoun(shown(name, line)) {
            found.push(format!("{name}:{}: {line}", number + 1));
        }
    }
}

#[test]
fn human_facing_texts_do_not_call_the_ai_she() {
    let mut found = Vec::new();
    for file in HUMAN_FACING {
        lines_with_pronouns(file, &resources().join(file), &mut found);
    }
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for file in IN_REPO {
        lines_with_pronouns(file, &repo.join(file), &mut found);
    }
    assert!(
        found.is_empty(),
        "给人看的字里还有代词：\n{}",
        found.join("\n")
    );
}

#[test]
fn the_check_catches_each_form() {
    for line in [
        "和她说话",
        "彼女に",
        "Talk with her",
        "while She sleeps",
        "HERSELF",
    ] {
        assert!(pronoun(line), "{line}");
    }
    for line in ["here", "there", "shell", "the AI", "AI と話す"] {
        assert!(!pronoun(line), "{line}");
    }
    // 页面的字只看值（施工 I-180）。
    let key = r#"    "activity/glance-hers": "[{where}] 下次 {at} 看一眼（AI 自己定的）","#;
    assert!(!pronoun(shown("packages/miyu-im/site/texts.js", key)));
    assert!(pronoun(shown(
        "packages/miyu-im/site/texts.js",
        r#"    "x": "她会看到","#
    )));
}
