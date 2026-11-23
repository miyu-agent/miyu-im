//! 录排函数的清单（施工 LW-IM-8，`cargo xtask order`，`docs/blueprint/onebot.md` 第一条「在哪」`xtask/src/order.rs` 一行）：设了 [`TRACE`] 的测试程序，
//! 核心拉起的 `miyu-im serve` 放进 QEMU 的用户态模拟器里跑，带 `-d page,in_asm`：模拟器只在一块代码头一回要执行时翻译它、把
//! 每条指令的地址写进日志，日志就是执行过的代码的全集，先后就是头一回执行的先后；`page` 先写一张内存布局，第一行是程序装在
//! 哪（地址照它换回程序里的）。
//!
//! 做法：测试程序旁边放的不是 `miyu-im` 的硬链接，而是一个同名的小脚本，子命令是 `serve` 的经模拟器跑真的程序，日志写进
//! [`TRACE`] 指的目录、一个进程一份（文件名是起来的时刻加进程号，照先后排）；别的子命令照常直接跑。只在 Linux 上做；没设的
//! 照原来硬链接（[`super::spawning::link_beside`]）。下一次没设变量跑测试时，发现旁边那个不是同一个文件，照常链回来。

use std::path::Path;

/// 录的日志放在哪个目录（绝对路径）：`cargo xtask order` 设，平时不设。
pub const TRACE: &str = "MIYU_IM_TRACE";

/// 经模拟器跑哪一份程序（绝对路径）：`cargo xtask order` 照发行那样另编的一份（测试编出来的那份依赖多开了几样功能，函数的
/// 符号名和发行的程序对不上）。没设的跑测试编出来的那份。
pub const PROGRAM: &str = "MIYU_IM_TRACE_PROGRAM";

/// 把 `source` 放到 `path`：设了 [`TRACE`] 的（Linux）放经模拟器跑的小脚本，别的硬链接。
pub fn beside(source: &Path, path: &Path) {
    match std::env::var_os(TRACE) {
        #[cfg(target_os = "linux")]
        Some(dir) => wrap(source, path, Path::new(&dir)),
        _ => super::spawning::link_beside(source, path),
    }
}

/// 写小脚本：先写到旁边一个只有这一次用的名字、给执行权限，再改名盖上 `path`（同 `link_beside`：并着跑的看到的要么是旧的、
/// 要么是新的）。
///
/// # Panics
///
/// 路径里有引号、`$`、反引号这类会把脚本拆坏的字符；写不进、改不了名。
#[cfg(target_os = "linux")]
fn wrap(source: &Path, path: &Path, dir: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let source =
        std::env::var_os(PROGRAM).map_or_else(|| source.to_path_buf(), std::path::PathBuf::from);
    let (source, dir) = (source.display().to_string(), dir.display().to_string());
    for text in [&source, &dir] {
        assert!(
            !text.contains(['"', '$', '`', '\\']),
            "路径里有会拆坏脚本的字符：{text}"
        );
    }
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = serve ]; then\n  exec qemu-x86_64 -d page,in_asm -D \"{dir}/$(date +%s%N)-$$.log\" \"{source}\" \"$@\"\nfi\nexec \"{source}\" \"$@\"\n"
    );
    let staged = path.with_extension(format!("tracing-{}", std::process::id()));
    std::fs::write(&staged, script).expect("写得进");
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).expect("改得了权限");
    std::fs::rename(&staged, path).expect("改得了名");
}
