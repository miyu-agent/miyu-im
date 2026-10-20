//! 改白名单（施工 I-13）。

use super::*;

fn qq() -> Platform {
    Platform::new("qq")
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
