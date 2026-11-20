//! 她发出去的文件、下载回执怎么认（施工 I-247、I-268）。

use super::*;
use crate::chat::{Id, Media};

/// 只带一个文件的段：编号 `id`、名字 `name`、`size` 个字节。
fn file(id: &str, name: &str, size: Option<u64>) -> Segments {
    Segments {
        media: vec![Media {
            kind: MediaKind::File,
            id: id.to_string(),
            name: Some(name.to_string()),
            size,
            fetch: None,
            url: None,
        }],
        ..Segments::default()
    }
}

/// 传成了的一份。
fn sent(name: &str, size: u64, id: Option<&str>, at: u64) -> Sent {
    Sent {
        name: name.to_string(),
        size,
        id: id.map(str::to_string),
        at,
    }
}

/// 测试自己的状态目录：先清空。
fn dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("im-sent-files-{}-{name}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("删得掉上一回的临时目录");
    }
    std::fs::create_dir_all(&dir).expect("建得了目录");
    dir
}

#[test]
fn a_message_with_only_one_file_is_a_receipt_candidate() {
    assert_eq!(
        receipt(&file("f", "say-01.mp3", Some(7))),
        Some(("f", "say-01.mp3", 7))
    );
    assert_eq!(
        receipt(&file("f", "say-01.mp3", None)),
        None,
        "没有字节数的不认"
    );
    let mut said = file("f", "say-01.mp3", Some(7));
    said.pieces = vec![Piece::Text("你看".to_string())];
    assert_eq!(receipt(&said), None, "带了字的是人发的");
    let mut blank = file("f", "say-01.mp3", Some(7));
    blank.pieces = vec![Piece::Text(" ".to_string())];
    assert_eq!(
        receipt(&blank),
        Some(("f", "say-01.mp3", 7)),
        "只有空白的照样认"
    );
    let mut at = file("f", "say-01.mp3", Some(7));
    at.pieces = vec![Piece::At(Id::from(1))];
    assert_eq!(receipt(&at), None);
    let mut quoting = file("f", "say-01.mp3", Some(7));
    quoting.reply_to = Some("9".to_string());
    assert_eq!(receipt(&quoting), None);
    let mut two = file("f", "a", Some(1));
    two.media.push(two.media[0].clone());
    assert_eq!(receipt(&two), None, "两样的不认");
}

#[test]
fn only_a_file_she_sent_here_lately_matches() {
    let files = SentFiles::default();
    let now = 1_000_000;
    files.note("qq:1:private:2", sent("say-01.mp3", 7, None, now));
    assert!(files.sent("qq:1:private:2", ("x", "say-01.mp3", 7), now));
    assert!(
        !files.sent("qq:1:private:2", ("x", "say-01.mp3", 8), now),
        "字节数不对"
    );
    assert!(
        !files.sent("qq:1:private:3", ("x", "say-01.mp3", 7), now),
        "别的场所"
    );
    assert!(
        !files.sent("qq:1:private:2", ("x", "say-01.mp3", 7), now + KEEP),
        "过了期限"
    );
}

/// 施工 I-268：平台回了文件编号的，编号对得上就是，名字、字节数读成别的也认；编号对不上、名字和字节数也对不上的不是。
#[test]
fn the_platforms_file_id_matches_on_its_own() {
    let files = SentFiles::default();
    let now = 1_000_000;
    files.note("v", sent("status.png", 1430029, Some("45df_fe5c"), now));
    assert!(files.sent("v", ("45df_fe5c", "status (1).png", 1), now));
    assert!(!files.sent("v", ("45df_0f80", "dp2.png", 1349829), now));
}

/// 施工 I-268（2026-11-20 主人报）：两天前传的文件、中间桥重启过，下载的回执照样认得出。
#[test]
fn what_she_sent_is_kept_across_a_restart_for_days() {
    let dir = dir("restart");
    let sent_at = 1_000_000;
    let two_days = 2 * 24 * 60 * 60 * 1000;
    SentFiles::load(dir.join(FILE)).note(
        "qq:1:private:2",
        sent("retry-resumed.png", 230668, Some("45df_ed6a"), sent_at),
    );
    let again = SentFiles::load(dir.join(FILE));
    assert!(again.sent(
        "qq:1:private:2",
        ("45df_ed6a", "retry-resumed.png", 230668),
        sent_at + two_days
    ));
    std::fs::remove_dir_all(&dir).expect("删得掉临时目录");
}

#[test]
fn each_venue_keeps_the_latest_few() {
    let files = SentFiles::default();
    let now = 1_000_000;
    for index in 0..=PER_VENUE {
        files.note(
            "v",
            sent(&format!("{index}.mp3"), 1, None, now + index as u64),
        );
    }
    let later = now + 60_000;
    assert!(!files.sent("v", ("x", "0.mp3", 1), later), "最早的挤出去了");
    assert!(files.sent("v", ("x", &format!("{PER_VENUE}.mp3"), 1), later));
}
