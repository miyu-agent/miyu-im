//! 测试用的出厂资源目录（施工 S-6 中）：核心的资源目录以后没有 `packages/onebot/`（核心施工 S-6 下删），测试要的「出厂的
//! onebot 包」照钉住的那份核心检出拼出来：抄一份核心的 `resources/`（门面 `testkit` 的 [`miyu_client::WORKSPACE_ROOT`]，
//! git 依赖在 cargo 的检出里是整个核心仓库），再把这个仓库的 `package/` 放成它的 `packages/onebot/`。核心里要是还带着一份
//! `packages/onebot/`（钉住的提交在 S-6 下以前），先删掉，用的总是这个仓库的。
//!
//! 一个测试程序拼一次（[`resources`]），放在 cargo 给集成测试的临时目录（`CARGO_TARGET_TMPDIR`）下、按进程分开。拷贝不用
//! 链接：核心只认真实路径，Windows 上建链接还要权限。拼之前把一小时以前的几份删掉，不随跑的次数越攒越多。

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use super::copy_tree;

/// 留着的旧拼法多久以后删：比一趟测试跑得久得多，删不到正在跑的别的测试程序的那一份。
const STALE: Duration = Duration::from_secs(60 * 60);

/// 这个仓库的包文件夹 `package/`。
pub fn package_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../package")
}

/// 拼好的资源目录：核心的出厂资源加上这个仓库的 `package/`（`packages/onebot/`）。一个测试程序第一次要时拼，以后照用。
pub fn resources() -> PathBuf {
    static ASSEMBLED: OnceLock<PathBuf> = OnceLock::new();
    ASSEMBLED.get_or_init(assemble).clone()
}

fn assemble() -> PathBuf {
    let parent = Path::new(env!("CARGO_TARGET_TMPDIR")).join("resources");
    sweep(&parent);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let dir = parent.join(format!("{}-{nanos}", std::process::id()));
    copy_tree(
        &Path::new(miyu_client::WORKSPACE_ROOT).join("resources"),
        &dir,
    );
    let onebot = dir.join("packages").join("onebot");
    if onebot.exists() {
        std::fs::remove_dir_all(&onebot).expect("删得掉核心里带的那一份");
    }
    copy_tree(&package_dir(), &onebot);
    dir
}

/// 删掉 `parent` 下一小时以前拼的。读不了、删不掉的留着：只是占地方，不影响测试。
fn sweep(parent: &Path) {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > STALE);
        if old && std::fs::remove_dir_all(entry.path()).is_err() {
            // 留着。
        }
    }
}
