//! 按群的设置（施工 Q-5）。

use super::*;

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("miyu-im-groups-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("删得了");
    }
    std::fs::create_dir_all(dir.join("admin")).expect("建得了");
    dir
}

#[test]
fn unwritten_groups_and_fields_take_the_shipped_settings() {
    let store = temp("defaults");
    let mut cached = Cached::default();
    assert_eq!(cached.group(&store, Id::from(31)), Group::default());
    assert!(!Group::default().spam.on, "出厂关着");
    std::fs::write(
        file(&store),
        r#"{"31": {"spam": {"on": true, "mute": 120}}}"#,
    )
    .expect("写得进");
    let group = cached.group(&store, Id::from(31));
    assert_eq!(
        group.spam,
        Spam {
            on: true,
            window: 60,
            mute: 120
        },
        "文件变了重读，没写的照出厂的"
    );
    assert_eq!(cached.group(&store, Id::from(32)), Group::default());
    std::fs::write(file(&store), "{broken").expect("写得进");
    let later = SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options()
        .write(true)
        .open(file(&store))
        .and_then(|handle| handle.set_modified(later))
        .expect("改得了时刻");
    assert_eq!(
        cached.group(&store, Id::from(31)),
        Group::default(),
        "写坏了照出厂的"
    );
    std::fs::remove_dir_all(&store).expect("删得了");
}
