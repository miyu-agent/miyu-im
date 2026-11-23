//! 程序 `miyu-im` 的链接参数（施工 LW-IM-4，`docs/blueprint/onebot.md` 第一条「在哪」`build.rs` 一行）：Linux x86_64（glibc）的
//! release 构建给 `miyu-im` 加 `-z pack-relative-relocs`（同核心 LW-2 的 `crates/miyu/build.rs`）。
//!
//! 位置无关的程序起来时，动态链接器要把整张重定位表读一遍、读过就留在内存里（`miyu-im` 的 `.rela.dyn` 有 400 KB）；打包成
//! RELR 以后只剩很小一块。要 glibc 2.36 以上：im 跟着核心发，最低要的核心在 Linux 上的基线是 glibc 2.39（核心的设计 12），
//! 发版的机器是 Ubuntu 24.04（2.39）。只加在 release 的 `miyu-im` 这一个程序上：日常的编译、测试、测试用的程序不变。
//!
//! 排函数（施工 LW-IM-8，同一行；同核心 LW-1）：环境变量 `MIYU_IM_LINK_ORDER` 指着一份清单（一行一个函数的符号名，照第一次
//! 执行的先后，`cargo xtask order` 录）时，给 `miyu-im` 的链接加 lld 的 `--symbol-ordering-file`，执行过的函数排在代码段最前面，
//! 常驻的代码页少一大半。没设的什么都不加，日常的编译、测试、门禁和原来一样。只做了 Linux；别的系统上设了的让构建失败：只给
//! 一句警告的话，编出来的程序没排，人却以为排了。清单的内容变了也要重链（`rerun-if-changed`）。
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
    println!("cargo:rerun-if-env-changed={ORDER}");
    match ordering(
        &cfg("CARGO_CFG_TARGET_OS"),
        std::env::var_os(ORDER).as_deref(),
    ) {
        Ok(lines) => lines.iter().for_each(|line| println!("{line}")),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

/// 排函数的清单的路径从这个环境变量读（施工 LW-IM-8）。
const ORDER: &str = "MIYU_IM_LINK_ORDER";

/// 排函数要给 cargo 的几行（施工 LW-IM-8）：没设变量的一行都没有。
fn ordering(os: &str, order: Option<&std::ffi::OsStr>) -> Result<Vec<String>, String> {
    let Some(order) = order else {
        return Ok(Vec::new());
    };
    if os != "linux" {
        return Err(format!(
            "{ORDER} 设了，但排函数只做了 Linux，这次编的目标系统是 {os}：不排的话不要设这个变量"
        ));
    }
    let path = std::path::Path::new(order);
    let Some(text) = path.to_str().filter(|_| path.is_absolute()) else {
        return Err(format!(
            "{ORDER} 要写清单的绝对路径（UTF-8），写的是 {}",
            path.display()
        ));
    };
    // `-Wl,` 照逗号把后面拆成几个参数，路径里有逗号的会被拆坏。
    if text.contains(',') {
        return Err(format!("{ORDER} 的路径 {text} 里有逗号：换一个目录"));
    }
    Ok(vec![
        format!("cargo:rerun-if-changed={text}"),
        format!("cargo:rustc-link-arg-bin=miyu-im=-Wl,--symbol-ordering-file={text}"),
        // 清单里有的名字这一次的程序里没有（改了代码、被内联掉的）时 lld 会逐个警告：不碍事，核对另在 `cargo xtask order` 里做。
        "cargo:rustc-link-arg-bin=miyu-im=-Wl,--no-warn-symbol-ordering".to_string(),
    ])
}

/// 这一次的构建开不开重定位打包：只有 Linux、x86_64、glibc 的 release（同核心）。aarch64 的链接器到 GNU ld 2.43 才认这个参数，
/// musl 的静态程序另是一回事，都没有量过，不开。
fn packs(os: &str, arch: &str, env: &str, profile: &str) -> bool {
    os == "linux" && arch == "x86_64" && env == "gnu" && profile == "release"
}
