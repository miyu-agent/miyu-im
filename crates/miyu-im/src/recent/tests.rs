//! 最近判断（施工 I-7 上补）：每个会话留几笔、几个会话并着读、读回存的那一份、正文的开头。

use serde_json::json;

use super::{MOST, Note, Recent, TEXT, lock};

fn note(at: u64, text: &str) -> Note {
    Note {
        at,
        who: "小林".into(),
        text: text.into(),
        outcome: "record".into(),
        why: json!({"reason": "quiet"}),
    }
}

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("miyu-im-recent-{name}-{}", std::process::id()));
    if std::fs::remove_dir_all(&dir).is_err() {
        // 没有就是没有。
    }
    dir
}

#[test]
fn each_venue_keeps_the_latest_and_several_merge_newest_first() {
    let recent = Recent::load(temp("keep").join("recent.json"));
    for at in 0..30 {
        recent.note("qq:30003:group:1", note(at * 2, "一"));
    }
    recent.note("qq:30003:group:2", note(41, "二"));
    recent.note("qq:30003:private:3", note(100, "三"));
    assert_eq!(
        lock(&recent.held)["qq:30003:group:1"].len(),
        MOST,
        "存的也只留这么多"
    );
    let one = recent.of(|venue| venue == "qq:30003:group:1");
    assert_eq!(one.len(), MOST);
    assert_eq!(one[0].1.at, 58, "新的在前");
    assert_eq!(one[MOST - 1].1.at, 20, "旧的去掉了");
    let groups = recent.of(|venue| venue.starts_with("qq:30003:group:"));
    assert_eq!(groups.len(), MOST, "并起来的也只交这么多");
    let ats: Vec<u64> = groups.iter().take(4).map(|(_, note)| note.at).collect();
    assert_eq!(ats, [58, 56, 54, 52]);
    assert!(
        groups
            .iter()
            .any(|(venue, note)| venue == "qq:30003:group:2" && note.at == 41)
    );
    assert!(recent.of(|venue| venue == "qq:30003:group:9").is_empty());
}

#[test]
fn the_written_file_is_read_back() {
    let dir = temp("file");
    std::fs::create_dir_all(&dir).expect("建得了目录");
    let file = dir.join("recent.json");
    let written = json!({"qq:30003:group:1": [note(5, "在吗")]});
    std::fs::write(&file, written.to_string()).expect("写得进");
    assert_eq!(
        Recent::load(file.clone()).of(|_| true)[0].1,
        note(5, "在吗")
    );
    std::fs::write(&file, b"not json").expect("写得进");
    assert!(
        Recent::load(file).of(|_| true).is_empty(),
        "读不懂的从空的记"
    );
    if std::fs::remove_dir_all(&dir).is_err() {
        // 删不掉就留在临时目录里。
    }
}

#[test]
fn the_sender_is_named_or_identified() {
    assert_eq!(Note::who(Some(" 小林 "), "qq:20004"), "小林");
    assert_eq!(Note::who(Some("  "), "qq:20004"), "qq:20004");
    assert_eq!(Note::who(None, "qq:20004"), "qq:20004");
}

#[test]
fn the_text_keeps_its_beginning() {
    assert_eq!(Note::excerpt("  有没有人\n懂  规则 "), "有没有人 懂 规则");
    let long = "字".repeat(TEXT + 5);
    let cut = Note::excerpt(&long);
    assert_eq!(cut.chars().count(), TEXT + 1);
    assert!(cut.ends_with('…'));
    assert_eq!(Note::excerpt(&"字".repeat(TEXT)), "字".repeat(TEXT));
}
