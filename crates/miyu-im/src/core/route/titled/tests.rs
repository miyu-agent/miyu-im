//! 群名（施工 O-49）：名字照发的人的名字洗、截到 64 个字符，空的没有（不是字的由驱动读成没有）；隔够了才再问。

use std::time::{Duration, Instant};

use super::{Titles, group_name};
use crate::chat::Platform;

const QQ: Platform = Platform::new("qq");
const TG: Platform = Platform::new("tg");

#[test]
fn the_group_name_is_cleaned_and_capped() {
    assert_eq!(group_name("摸鱼群"), Some("摸鱼群".to_string()));
    assert_eq!(group_name("a\nb").as_deref(), Some("ab"));
    let long = "群".repeat(80);
    assert_eq!(group_name(&long).map(|name| name.chars().count()), Some(64));
    assert_eq!(group_name("  "), None);
}

#[test]
fn a_group_is_asked_again_only_after_the_interval() {
    let mut titles = Titles::new(Duration::from_secs(60));
    let now = Instant::now();
    assert!(titles.due((QQ, 1), now), "没问过的问");
    assert!(
        !titles.due((QQ, 1), now + Duration::from_secs(59)),
        "不到一分钟不问"
    );
    assert!(titles.due((QQ, 2), now), "各群各算");
    assert!(
        titles.due((TG, 1), now),
        "两个平台同号的群各算（施工 I-70 上）"
    );
    assert!(
        titles.due((QQ, 1), now + Duration::from_secs(60)),
        "到了再问"
    );
    let mut every = Titles::new(Duration::ZERO);
    assert!(
        every.due((QQ, 1), now) && every.due((QQ, 1), now),
        "0 是每次都问"
    );
}
