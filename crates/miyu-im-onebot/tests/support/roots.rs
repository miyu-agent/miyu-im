//! 测试的临时数据根（从 `mod.rs` 挪来：那个文件到了行数上限，施工 I-35）：建好骨架。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use miyu_store::env::{Env, Platform};
use miyu_store::root::DataRoot;

/// 一个新的临时数据根（`MIYU_HOME` 指到它），建好骨架：交回目录（用完调的一方删）和数据根。
pub fn temp_root() -> (PathBuf, DataRoot) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let dir = std::env::temp_dir().join(format!(
        "miyu-im-{}-{nanos}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let root = DataRoot::locate(&Env {
        platform: Platform::current(),
        miyu_home: Some(dir.clone().into_os_string()),
        home: None,
        xdg_cache_home: None,
        local_app_data: None,
        miyu_resources: None,
        exe: None,
    })
    .expect("MIYU_HOME 是绝对路径");
    root.prepare().expect("临时目录里建得了骨架");
    (dir, root)
}

/// 主人的家目录：人格、预设、技能、脚本只认这里。核心 SU-5 上起数据根只剩一层，就是数据根本身（原来是 `home/<主人>/`）。
pub fn owner_home(root: &DataRoot) -> PathBuf {
    root.path().to_path_buf()
}
