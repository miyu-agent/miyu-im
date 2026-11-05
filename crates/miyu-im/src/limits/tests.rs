//! 使用限制：三条线碰到哪一条算满、没写的不限；每人的设置照文件读，单独设的整组替换默认的，写错的当空的。

use super::*;

/// 一个空的临时目录。
fn dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("im-limits-{}-{name}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("删得掉上一回留下的");
    }
    std::fs::create_dir_all(&dir).expect("建得了");
    dir
}

#[test]
fn any_line_reached_is_full_and_unwritten_lines_never_are() {
    let lines = Lines {
        limit_5h: Some(100),
        limit_week: None,
        limit_month: Some(1000),
    };
    assert!(
        !lines.reached([99, u64::MAX, 999]),
        "没写的那条不管多少都不算"
    );
    assert!(lines.reached([100, 0, 0]), "正好碰到算满");
    assert!(lines.reached([0, 0, 1000]));
    assert!(!Lines::default().reached([u64::MAX; 3]));
    assert!(Lines::default().is_empty());
    assert!(!lines.is_empty());
    assert_eq!(Lines::from_caps(lines.caps()), lines);
}

#[test]
fn a_listed_person_replaces_the_default_as_a_whole() {
    let people: People = toml::from_str(
        "[default]\nlimit_5h = 10\nlimit_week = 20\n\n[[person]]\nid = \"qq:2\"\nlimit_month = 30\n\n[[person]]\nid = \"qq:3\"\n",
    )
    .expect("合写法");
    assert_eq!(
        people.lines_for("qq:1"),
        Lines {
            limit_5h: Some(10),
            limit_week: Some(20),
            limit_month: None
        }
    );
    assert_eq!(
        people.lines_for("qq:2"),
        Lines {
            limit_month: Some(30),
            ..Lines::default()
        },
        "不跟默认的逐条合并"
    );
    assert!(people.lines_for("qq:3").is_empty(), "三条都不写的不限");
}

#[test]
fn a_missing_or_broken_file_limits_nobody_and_a_change_is_picked_up() {
    let dir = dir("watched");
    let path = dir.join(FILE);
    assert_eq!(People::read(&path), People::default(), "没有文件");
    std::fs::write(&path, "[default]\nlimit_5h = \"many\"\n").expect("写得了");
    assert_eq!(People::read(&path), People::default(), "写错的");
    let mut watched = Watched::new(path.clone());
    assert!(watched.people().default.is_empty());
    std::fs::write(&path, "[default]\nlimit_5h = 5\n").expect("写得了");
    // 改的时刻要变：有的文件系统只记到秒。
    let later = SystemTime::now() + Duration::from_secs(2);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .and_then(|file| file.set_modified(later))
        .expect("改得了时刻");
    watched.checked = None;
    assert_eq!(watched.people().default.limit_5h, Some(5));
    std::fs::remove_dir_all(dir).expect("删得掉临时目录");
}
