use super::*;

const TABLE: &str = "\
### 登记簿

| 文件 | 进到哪 | 什么时候加进来 | token | 为什么加 | 指纹 |
|---|---|---|---|---|---|
| `im/data/a.txt` | 事实 | 断了的那一次 | 3 | 施工 O-1 | `2cf24dba` |
| `im/data/b.txt` | 事实 | 每次 | 1 | 施工 O-1 | `00000000` |

### 附录
";

fn files(entries: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>> {
    entries
        .iter()
        .map(|(path, text)| ((*path).to_string(), text.as_bytes().to_vec()))
        .collect()
}

#[test]
fn the_fingerprint_is_the_first_eight_hex_of_the_sha256() {
    assert_eq!(fingerprint(b"hello"), "2cf24dba");
}

#[test]
fn the_rows_are_read_from_the_section_table() {
    let recorded = rows(TABLE).unwrap();
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded["im/data/a.txt"], "2cf24dba");
    assert!(rows("### 别的\n").is_err());
    let twice = TABLE.replace("`im/data/b.txt`", "`im/data/a.txt`");
    assert!(rows(&twice).unwrap_err().contains("登记了两行"));
    let short = TABLE.replace(" | 施工 O-1 | `00000000` |", " | `00000000` |");
    assert!(rows(&short).unwrap_err().contains("要有 6 格"));
}

#[test]
fn every_mismatch_is_named() {
    let recorded = rows(TABLE).unwrap();
    // a 对上了；b 的字变了；c 没登记。
    let problems = compare(
        &recorded,
        &files(&[
            ("im/data/a.txt", "hello"),
            ("im/data/b.txt", "x"),
            ("im/data/c.txt", "y"),
        ]),
    );
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems[0].contains("im/data/b.txt 的字变了"));
    assert!(problems[1].contains("im/data/c.txt 没登记"));
    // 登记了却没有文件。
    let problems = compare(&recorded, &files(&[("im/data/a.txt", "hello")]));
    assert_eq!(
        problems,
        ["登记簿里有 im/data/b.txt，im/data/ 下却没有这份文件"]
    );
}

#[test]
fn human_texts_and_the_exempt_data_are_not_walked() {
    let dir = std::env::temp_dir().join(format!("miyu-im-ledger-{}", std::process::id()));
    for (path, text) in [
        ("judge/system.txt", "s"),
        ("tools/recall.json", "{}"),
        ("human/en.json", "{}"),
        ("judge/human/zh.json", "{}"),
        ("bridge.json", "{}"),
        ("defaults.toml", "[chatty]"),
        ("moderation.txt", "spam"),
        ("venues.d/50-defaults.toml", "[[rule]]"),
        ("venues.d/60-more.toml", "[[rule]]"),
        ("judge/defaults.toml", "d"),
        ("tools/bridge.json", "{}"),
        ("prompt.txt", "p"),
    ] {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let mut found = BTreeMap::new();
    let walked = walk(&dir, "im/data/", &mut found);
    std::fs::remove_dir_all(&dir).unwrap();
    walked.unwrap();
    // 给人看的字（哪一层的 human/ 都是）、正好那四份数据不登记；别处同名的、venues.d/ 里别的照查。
    assert_eq!(
        found.keys().collect::<Vec<_>>(),
        [
            "im/data/judge/defaults.toml",
            "im/data/judge/system.txt",
            "im/data/prompt.txt",
            "im/data/tools/bridge.json",
            "im/data/tools/recall.json",
            "im/data/venues.d/60-more.toml",
        ]
    );
}
