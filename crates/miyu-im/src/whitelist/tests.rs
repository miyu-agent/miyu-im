//! 改白名单（施工 I-13）。

use super::*;

/// 只挂着 QQ。
const QQ: &[Platform] = &[Platform::new("qq")];

/// QQ 在前、Telegram 在后。
const BOTH: &[Platform] = &[Platform::new("qq"), Platform::new("tg")];

fn qq() -> &'static [Platform] {
    QQ
}

#[test]
fn numbers_are_added_once_and_removed_when_there() {
    let listed = vec!["qq:10001".to_string()];
    assert_eq!(
        changed(&listed, &Change::Add(" 20002 ".to_string()), qq()),
        Ok((
            Some(vec!["qq:10001".to_string(), "qq:20002".to_string()]),
            Some(Said::Added("qq:20002".to_string()))
        ))
    );
    assert_eq!(
        changed(&listed, &Change::Add("10001".to_string()), qq()),
        Ok((None, Some(Said::Already("qq:10001".to_string()))))
    );
    assert_eq!(
        changed(&listed, &Change::Remove("10001".to_string()), qq()),
        Ok((
            Some(Vec::new()),
            Some(Said::Removed("qq:10001".to_string()))
        ))
    );
    assert_eq!(
        changed(&listed, &Change::Remove("3".to_string()), qq()),
        Ok((None, Some(Said::Absent("qq:3".to_string()))))
    );
    for bad in ["0", "-1", "abc", ""] {
        assert_eq!(
            changed(&listed, &Change::Add(bad.to_string()), qq()),
            Err(Said::Bad(bad.to_string()))
        );
    }
    assert_eq!(
        changed(&[], &Change::List, qq()),
        Ok((None, Some(Said::Empty)))
    );
    assert_eq!(changed(&listed, &Change::List, qq()), Ok((None, None)));
}

#[test]
fn a_number_may_name_its_platform() {
    // 施工 AUD-4（审计 #114）：写成 `<平台>:<号>` 的照那个平台，只写号的照头一个；没挂着的平台、写错的不认。
    assert_eq!(
        changed(&[], &Change::Add("tg:42".to_string()), BOTH),
        Ok((
            Some(vec!["tg:42".to_string()]),
            Some(Said::Added("tg:42".to_string()))
        ))
    );
    assert_eq!(
        changed(&[], &Change::Add("42".to_string()), BOTH),
        Ok((
            Some(vec!["qq:42".to_string()]),
            Some(Said::Added("qq:42".to_string()))
        ))
    );
    for bad in ["tg:42", "wx:1", "tg:", ":1"] {
        assert_eq!(
            changed(&[], &Change::Add(bad.to_string()), QQ),
            Err(Said::Bad(bad.to_string())),
            "{bad}"
        );
    }
    assert_eq!(
        number_of("tg:-1001234567890", BOTH, true),
        Some((Platform::new("tg"), "-1001234567890".to_string()))
    );
    assert_eq!(number_of("-5", BOTH, false), None, "号不是负的");
    assert_eq!(number_of("0", BOTH, true), None);
}
