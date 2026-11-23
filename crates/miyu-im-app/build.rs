//! 程序 `miyu-im` 的链接参数（施工 LW-IM-4，`docs/blueprint/onebot.md` 第一条「在哪」`build.rs` 一行）：Linux x86_64（glibc）的
//! release 构建给 `miyu-im` 加 `-z pack-relative-relocs`（同核心 LW-2 的 `crates/miyu/build.rs`）。
//!
//! 位置无关的程序起来时，动态链接器要把整张重定位表读一遍、读过就留在内存里（`miyu-im` 的 `.rela.dyn` 有 400 KB）；打包成
//! RELR 以后只剩很小一块。要 glibc 2.36 以上：im 跟着核心发，最低要的核心在 Linux 上的基线是 glibc 2.39（核心的设计 12），
//! 发版的机器是 Ubuntu 24.04（2.39）。只加在 release 的 `miyu-im` 这一个程序上：日常的编译、测试、测试用的程序不变。
//!
//! 为什么用构建脚本不用 `RUSTFLAGS`：`RUSTFLAGS` 一变所有依赖都要重编，还会带到构建脚本和过程宏上；`rustc-link-arg-bin` 只加在
//! 这一个程序的链接上。

fn main() {
    let cfg = |name: &str| std::env::var(name).unwrap_or_default();
    if packs(
        &cfg("CARGO_CFG_TARGET_OS"),
        &cfg("CARGO_CFG_TARGET_ARCH"),
        &cfg("CARGO_CFG_TARGET_ENV"),
        &cfg("PROFILE"),
    ) {
        println!("cargo:rustc-link-arg-bin=miyu-im=-Wl,-z,pack-relative-relocs");
    }
}

/// 这一次的构建开不开重定位打包：只有 Linux、x86_64、glibc 的 release（同核心）。aarch64 的链接器到 GNU ld 2.43 才认这个参数，
/// musl 的静态程序另是一回事，都没有量过，不开。
fn packs(os: &str, arch: &str, env: &str, profile: &str) -> bool {
    os == "linux" && arch == "x86_64" && env == "gnu" && profile == "release"
}
