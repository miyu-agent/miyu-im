//! 平台能力的名字（施工 I-8 上）：每一样一个名字，照名字认得回来，认不出的没有。

use super::{CAPS, Cap};

#[test]
fn every_cap_reads_back_from_its_name() {
    for cap in CAPS {
        assert_eq!(Cap::parse(cap.name()), Some(cap), "{cap:?}");
    }
    let mut names: Vec<&str> = CAPS.iter().map(|cap| cap.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), CAPS.len(), "名字不重");
    assert_eq!(Cap::parse("Recall"), None, "照原样认，不改大小写");
    assert_eq!(Cap::parse("fly"), None);
}
