use super::*;

/// 绝对路径的样子随系统：Windows 上要带盘符。
fn absolute(tail: &str) -> String {
    if cfg!(windows) {
        format!(r"C:\{tail}")
    } else {
        format!("/{tail}")
    }
}

#[test]
fn the_book_follows_the_state_dir_the_core_gives() {
    let (new, old) = (absolute("new"), absolute("old"));
    let both = |key: &str| match key {
        "MIYU_STATE_DIR" => Some(OsString::from(&new)),
        "MIYU_PACKAGE_STATE" => Some(OsString::from(&old)),
        _ => None,
    };
    assert_eq!(
        within("sponsors.json", both),
        PathBuf::from(&new).join("sponsors.json")
    );
    let only_old = |key: &str| (key == "MIYU_PACKAGE_STATE").then(|| OsString::from(&old));
    assert_eq!(
        within("sponsors.json", only_old),
        PathBuf::from(&old).join("sponsors.json")
    );
}

#[test]
fn without_an_absolute_state_dir_the_book_stays_in_the_working_directory() {
    let relative = |key: &str| (key == "MIYU_STATE_DIR").then(|| OsString::from("state"));
    assert_eq!(
        within("sponsors.json", relative),
        PathBuf::from("sponsors.json")
    );
    assert_eq!(
        within("sponsors.json", |_| None),
        PathBuf::from("sponsors.json")
    );
}
