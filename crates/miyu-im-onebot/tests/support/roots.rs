//! 测试的临时数据根（从 `mod.rs` 挪来：那个文件到了行数上限，施工 I-35）：建好骨架、写一个机主账号。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use miyu_kernel::id::AccountId;
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
    owner(&root);
    (dir, root)
}

/// 写一个机主账号（施工 I-35，核心 Y-12 中上起：没有机主的数据根核心停在设置那一段，本机令牌握手回 `setup: true`）。照核心测试的
/// `owner()`：编号 `admin`，标 `owner`。
fn owner(root: &DataRoot) {
    use miyu_store::accounts::{self, Account, Accounts};
    let mut written = Accounts::default();
    written.put(Account {
        id: "admin".to_string(),
        username: "admin".to_string(),
        // 核心 SU-1 起自己建的主人就是不设访问密码的。
        password: String::new(),
        changed: miyu_kernel::time::Timestamp::parse("2026-10-23T00:00:00.000Z").expect("合写法"),
        created: None,
        disabled: false,
        owner: true,
    });
    accounts::write(&root.system().join(accounts::FILE), &written, None).expect("写得进");
}

/// 主人的编号（`owner()` 写的那一个）。
pub fn owner_id() -> AccountId {
    AccountId::parse("admin").expect("合写法")
}

/// 主人的家目录：核心 SU-3 下起人格、预设、技能、脚本只认这里（`home/<主人>/…`），系统区的不读。
pub fn owner_home(root: &DataRoot) -> PathBuf {
    root.account_dir(&owner_id())
}
