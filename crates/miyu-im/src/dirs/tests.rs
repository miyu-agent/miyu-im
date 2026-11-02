use super::*;

/// 照一张表读变量。
fn table(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| OsString::from(value))
    }
}

/// 绝对路径的样子随系统：Windows 上要带盘符。
fn absolute(tail: &str) -> &'static str {
    let text = if cfg!(windows) {
        format!(r"C:\{tail}")
    } else {
        format!("/{tail}")
    };
    Box::leak(text.into_boxed_str())
}

#[test]
fn the_new_name_wins_over_the_old_one() {
    let (new, old) = (absolute("new"), absolute("old"));
    let pairs: &'static [(&str, &str)] =
        Box::leak(vec![("MIYU_STATE_DIR", new), ("MIYU_PACKAGE_STATE", old)].into_boxed_slice());
    assert_eq!(given(STATE, table(pairs)), Some(PathBuf::from(new)));
}

#[test]
fn the_old_name_still_works_while_the_core_gives_both() {
    let old = absolute("old");
    let pairs: &'static [(&str, &str)] =
        Box::leak(vec![("MIYU_PACKAGE_STATE", old)].into_boxed_slice());
    assert_eq!(given(STATE, table(pairs)), Some(PathBuf::from(old)));
}

#[test]
fn a_relative_or_empty_value_counts_as_not_given() {
    let pairs: &'static [(&str, &str)] = &[("MIYU_STATE_DIR", ""), ("MIYU_PACKAGE_STATE", "state")];
    assert_eq!(given(STATE, table(pairs)), None);
    assert_eq!(given(LOG, table(&[])), None);
}
