//! 她发出去的文件、下载回执怎么认（施工 I-247）。

use std::time::{Duration, Instant};

use super::*;
use crate::chat::{Id, Media};

/// 只带一个文件的段：名字 `name`、`size` 个字节。
fn file(name: &str, size: Option<u64>) -> Segments {
    Segments {
        media: vec![Media {
            kind: MediaKind::File,
            id: "f".to_string(),
            name: Some(name.to_string()),
            size,
            fetch: None,
            url: None,
        }],
        ..Segments::default()
    }
}

#[test]
fn a_message_with_only_one_file_is_a_receipt_candidate() {
    assert_eq!(
        receipt(&file("say-01.mp3", Some(7))),
        Some(("say-01.mp3", 7))
    );
    assert_eq!(receipt(&file("say-01.mp3", None)), None, "没有字节数的不认");
    let mut said = file("say-01.mp3", Some(7));
    said.pieces = vec![Piece::Text("你看".to_string())];
    assert_eq!(receipt(&said), None, "带了字的是人发的");
    let mut blank = file("say-01.mp3", Some(7));
    blank.pieces = vec![Piece::Text(" ".to_string())];
    assert_eq!(receipt(&blank), Some(("say-01.mp3", 7)), "只有空白的照样认");
    let mut at = file("say-01.mp3", Some(7));
    at.pieces = vec![Piece::At(Id::from(1))];
    assert_eq!(receipt(&at), None);
    let mut quoting = file("say-01.mp3", Some(7));
    quoting.reply_to = Some("9".to_string());
    assert_eq!(receipt(&quoting), None);
    let mut two = file("a", Some(1));
    two.media.push(two.media[0].clone());
    assert_eq!(receipt(&two), None, "两样的不认");
}

#[test]
fn only_a_file_she_sent_here_lately_matches() {
    let sent = SentFiles::default();
    let now = Instant::now();
    sent.note("qq:1:private:2", ("say-01.mp3", 7), now);
    assert!(sent.sent("qq:1:private:2", ("say-01.mp3", 7), now));
    assert!(
        !sent.sent("qq:1:private:2", ("say-01.mp3", 8), now),
        "字节数不对"
    );
    assert!(
        !sent.sent("qq:1:private:3", ("say-01.mp3", 7), now),
        "别的场所"
    );
    assert!(
        !sent.sent("qq:1:private:2", ("say-01.mp3", 7), now + KEEP),
        "过了期限"
    );
}

#[test]
fn each_venue_keeps_the_latest_few() {
    let sent = SentFiles::default();
    let now = Instant::now();
    for index in 0..=PER_VENUE {
        sent.note(
            "v",
            (&format!("{index}.mp3"), 1),
            now + Duration::from_secs(index as u64),
        );
    }
    let later = now + Duration::from_secs(60);
    assert!(!sent.sent("v", ("0.mp3", 1), later), "最早的挤出去了");
    assert!(sent.sent("v", (&format!("{PER_VENUE}.mp3"), 1), later));
}
