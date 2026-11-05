//! `chat.media` 分段交（施工 I-154）：哪一段、交完没有；不带 `offset` 的大图拒；留着的几张。

use std::sync::Arc;

use serde_json::json;

use super::{PAGE, Recent, Taken, page};

#[test]
fn pages_cover_the_whole_picture() {
    let size = 2 * PAGE + 7;
    assert_eq!(page(size, Some(0)), Ok((0..PAGE, false)));
    assert_eq!(page(size, Some(PAGE as u64)), Ok((PAGE..2 * PAGE, false)));
    assert_eq!(
        page(size, Some(2 * PAGE as u64)),
        Ok((2 * PAGE..size, true))
    );
    assert_eq!(
        page(size, Some(size as u64)),
        Ok((size..size, true)),
        "正好在末尾的交空的、交完了"
    );
    assert_eq!(page(size, Some(size as u64 + 1)), Err("bad_params"));
}

#[test]
fn without_an_offset_only_a_small_picture_comes_whole() {
    assert_eq!(page(PAGE, None), Ok((0..PAGE, true)));
    assert_eq!(
        page(PAGE + 1, None),
        Err("too_large"),
        "大的拒，不再整张塞进一行"
    );
}

#[test]
fn recent_pictures_are_kept_and_the_oldest_dropped() {
    let recent = Recent::default();
    let taken = |n: u8| {
        Arc::new(Taken {
            bytes: vec![n],
            answer: json!({}),
        })
    };
    for n in 0..5 {
        recent.put(format!("k{n}"), taken(n));
    }
    assert!(recent.get("k0").is_none(), "最多留四张，最早的扔掉");
    assert_eq!(recent.get("k4").expect("留着").bytes, vec![4]);
}
