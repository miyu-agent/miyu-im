//! 命令行文案的规矩（施工 I-87，照核心设计 47 第三节第 10 条、核心 CL-3）：`miyu im` 印给人看的字不以句号结尾，中文里没有「你」「她」，
//! 帮助页每条说明不用「、」串；三种语言的帮助页一个命令一份。程序名前缀、「提示：」由代码拼（`texts/cli.rs`），不在字里。

use std::collections::BTreeMap;

use serde_json::Value;

/// 命令行会印的那几类字的开头（`said` 里的键）。
const CLI: &[&str] = &[
    "help/",
    "cli/",
    "control/",
    "status/",
    "reason/",
    "whitelist/",
    "access/",
    "web/",
    "failure/",
    "logs/",
    "notice/",
    "no-log",
    "venue/none",
    "venue/defaults",
    "venue/bad-venue",
];

/// 帮助页要有的几份。
const PAGES: &[&str] = &[
    "main",
    "start",
    "stop",
    "restart",
    "status",
    "web",
    "logs",
    "venue",
    "whitelist",
    "access",
];

/// 语言 `language` 的 `said`。
fn said(language: &str) -> BTreeMap<String, String> {
    let path = crate::support::assembled::package_dir()
        .join("data/human")
        .join(format!("{language}.json"));
    let text = std::fs::read_to_string(&path).expect("读得到");
    let all: Value = serde_json::from_str(&text).expect("是 JSON");
    all["said"]
        .as_object()
        .expect("有 said")
        .iter()
        .map(|(key, value)| (key.clone(), value.as_str().unwrap_or_default().to_string()))
        .collect()
}

#[test]
fn command_line_words_follow_the_cli_rules() {
    for language in ["zh", "en", "ja"] {
        for (key, text) in said(language) {
            if !CLI.iter().any(|head| key.starts_with(head)) {
                continue;
            }
            for line in text.lines() {
                let line = line.trim_end();
                assert!(
                    !line.ends_with('。') && !line.ends_with('.'),
                    "{language} {key} 以句号结尾：{line}"
                );
            }
            if language == "zh" {
                assert!(
                    !text.contains('你') && !text.contains('她'),
                    "{language} {key} 有「你」「她」：{text}"
                );
            }
        }
    }
}

#[test]
fn every_command_has_a_help_page_whose_descriptions_are_not_lists() {
    for language in ["zh", "en", "ja"] {
        let said = said(language);
        for page in PAGES {
            let text = said
                .get(&format!("help/{page}"))
                .unwrap_or_else(|| panic!("{language} 没有 help/{page}"));
            // 两格空格开头的是表里的一行：说明那一列不用「、」串几个动作。
            for row in text.lines().filter(|line| line.starts_with("  ")) {
                let description = row.trim().split("  ").last().unwrap_or_default();
                assert!(
                    !description.contains('、'),
                    "{language} help/{page} 的说明是「、」串：{row}"
                );
            }
        }
    }
}
