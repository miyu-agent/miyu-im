//! 排函数（施工 LW-IM-8，`docs/blueprint/onebot.md` 第一条「在哪」`xtask/src/order.rs` 一行；照核心 LW-1）：`cargo xtask order` 编出照执行先后排过
//! 代码段的 release 程序 `target/release/miyu-im`，清单写在 `target/order/miyu-im.txt`。
//!
//! 1. 清单先写成空文件，带着 `MIYU_IM_LINK_ORDER`（`crates/miyu-im-app/build.rs`）照发行那样编 release 的程序、拷一份另放
//!    （录的是它：编测试时依赖多开了几样功能，函数的符号名和发行的程序对不上），再编 release 的集成测试。
//! 2. 照 [`PHASES`] 一段一段跑那几组集成测试（release），带着 `MIYU_IM_TRACE`：核心拉起的 `miyu-im serve` 在 QEMU 的用户态
//!    模拟器里跑，每个进程一份日志（`crates/miyu-im-app/tests/support/traced.rs`）。只有桥自己在模拟器里：测试程序、进程内的
//!    核心照常原生跑，清单里只有 `miyu-im` 的函数。日志照 `nm` 换成函数（`order/listing.rs`）：头一段照第一次执行的先后，
//!    后面每段只把新见到的接在后面，不常走的路径整段排在最后，不会插进常走的中间。每段换完就删日志（一份几十 MB）。
//! 3. 写好清单再编一遍程序：构建脚本照清单的内容重跑，只有 `miyu-im` 重新链接。
//! 4. 核对排上了（`order/verify.rs`，九成线）。录不出、模拟器没装、排不上都报错退出：不交一个没排的程序当排过的。
//!
//! 段里的测试有失败的不算错（模拟器里慢，个别等时刻的测试可能等不到），只打一句；一段一个函数都没录到的报错。

mod listing;
mod verify;

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

use listing::{Listing, functions};

/// 录的几段：名字和交给测试程序的过滤词（测试路径里含其中一个的跑）。先常走的，后不常走的。
const PHASES: [(&str, &[&str]); 5] = [
    (
        "群聊",
        &[
            "group::",
            "called::",
            "judged::",
            "replies::",
            "quote::",
            "mention::",
            "private::",
            "revived::",
        ],
    ),
    (
        "取图和文件",
        &["media::", "pictures::", "send_file::", "lookup::"],
    ),
    ("页面", &["site", "page::", "web::"]),
    ("Telegram", &["telegram::", "telegram_"]),
    (
        "管理和看一眼",
        &["admin", "members::", "glance::", "commands::"],
    ),
];

/// 清单的环境变量（同 `crates/miyu-im-app/build.rs`）。
const ORDER: &str = "MIYU_IM_LINK_ORDER";

/// 录日志的环境变量（同 `crates/miyu-im-app/tests/support/traced.rs`）。
const TRACE: &str = "MIYU_IM_TRACE";

/// 经模拟器跑哪一份程序（同上）：发行那样编的那一份的拷贝。
const TRACED: &str = "MIYU_IM_TRACE_PROGRAM";

/// 跑一遍（见模块的说明）。交回问题，空的是成了。
pub(crate) fn run(root: &Path, cargo: &str) -> Result<(), String> {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return Err("排函数只做了 Linux x86_64".to_string());
    }
    for tool in ["qemu-x86_64", "nm"] {
        let found = Command::new(tool).arg("--version").output();
        if !found.is_ok_and(|output| output.status.success()) {
            return Err(format!("找不到 {tool}：排函数要它（qemu-user、binutils）"));
        }
    }
    let dir = root.join("target").join("order");
    let list = dir.join("miyu-im.txt");
    let program = root.join("target").join("release").join("miyu-im");
    std::fs::create_dir_all(&dir).map_err(|e| format!("建不了 {}：{e}", dir.display()))?;
    write(&list, "")?;
    // 录的是发行那样编的程序（不带测试才开的依赖的功能：带了的话函数的符号名不一样，清单对不上发行的程序），拷一份另放：
    // 接着编测试时 cargo 会照测试的那一套重编 `target/release/miyu-im`。
    let build = [
        "build",
        "--release",
        "--locked",
        "-p",
        "miyu-im-app",
        "--bin",
        "miyu-im",
    ];
    cargo_run(cargo, root, &list, None, &build)?;
    let recorded = dir.join("miyu-im.recorded");
    std::fs::copy(&program, &recorded).map_err(|e| format!("拷不了 {}：{e}", program.display()))?;
    cargo_run(
        cargo,
        root,
        &list,
        None,
        &[
            "test",
            "--release",
            "--locked",
            "-p",
            "miyu-im-app",
            "--test",
            "all",
            "--no-run",
        ],
    )?;
    let symbols = functions(&nm(&recorded, false)?);
    let mut listing = Listing::new(&symbols);
    for (name, filters) in PHASES {
        let logs = dir.join("logs");
        if logs.exists() {
            std::fs::remove_dir_all(&logs)
                .map_err(|e| format!("删不了 {}：{e}", logs.display()))?;
        }
        std::fs::create_dir_all(&logs).map_err(|e| format!("建不了 {}：{e}", logs.display()))?;
        let before = listing.count();
        let mut args = vec![
            "test",
            "--release",
            "--locked",
            "-p",
            "miyu-im-app",
            "--test",
            "all",
            "--",
        ];
        args.extend_from_slice(filters);
        args.push("--test-threads=2");
        if let Err(error) = cargo_run(cargo, root, &list, Some((&logs, &recorded)), &args) {
            println!("（{name} 这一段有测试没过，只影响录到多少：{error}）");
        }
        let read = read_logs(&logs, &mut listing)?;
        std::fs::remove_dir_all(&logs).map_err(|e| format!("删不了 {}：{e}", logs.display()))?;
        if read == 0 || listing.count() == 0 {
            return Err(format!(
                "{name} 这一段一份日志都没录到：桥没在模拟器里起来？"
            ));
        }
        println!(
            "── {name}：新录到 {} 个函数，累计 {} 个、{} 字节 ──",
            listing.count() - before,
            listing.count(),
            listing.bytes()
        );
    }
    let text: String = listing.names().map(|name| format!("{name}\n")).collect();
    write(&list, &text)?;
    cargo_run(
        cargo,
        root,
        &list,
        None,
        &[
            "build",
            "--release",
            "--locked",
            "-p",
            "miyu-im-app",
            "--bin",
            "miyu-im",
        ],
    )?;
    let placed = verify::check(&nm(&program, true)?, &text)?;
    println!(
        "排好了：清单 {} 个函数，符号表里找到 {} 个、合计 {} 字节，{}% 排在代码段开头；程序 {}，清单 {}",
        placed.listed,
        placed.found,
        placed.bytes,
        placed.near * 100 / placed.bytes,
        program.display(),
        list.display()
    );
    Ok(())
}

/// 目录 `logs` 里的日志照文件名（起来的时刻）的先后收进 `listing`，交回几份。
fn read_logs(logs: &Path, listing: &mut Listing<'_>) -> Result<usize, String> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(logs)
        .map_err(|e| format!("读不了 {}：{e}", logs.display()))?
        .flatten()
        .map(|entry| entry.path())
        .collect();
    found.sort();
    let count = found.len();
    for path in found {
        let file =
            std::fs::File::open(&path).map_err(|e| format!("读不了 {}：{e}", path.display()))?;
        let mut base = None;
        let mut head = String::new();
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|e| format!("读不了 {}：{e}", path.display()))?;
            match base {
                Some(base) => listing.line(base, &line),
                None => {
                    head.push_str(&line);
                    head.push('\n');
                    base = listing::base(&head);
                }
            }
        }
    }
    Ok(count)
}

/// `nm --defined-only -S`（`sorted` 的照地址排）这个程序。
fn nm(program: &Path, sorted: bool) -> Result<String, String> {
    let mut command = Command::new("nm");
    command.args(["--defined-only", "-S"]);
    if sorted {
        command.arg("--numeric-sort");
    }
    let output = command
        .arg(program)
        .output()
        .map_err(|e| format!("跑不了 nm：{e}"))?;
    if !output.status.success() {
        return Err(format!(
            "nm {} 没成：{}",
            program.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// 带着清单跑一条 cargo；录的那几段另带录日志的目录和经模拟器跑的程序。
fn cargo_run(
    cargo: &str,
    root: &Path,
    list: &Path,
    traced: Option<(&Path, &Path)>,
    args: &[&str],
) -> Result<(), String> {
    println!(
        "\n── {ORDER}={} cargo {} ──",
        list.display(),
        args.join(" ")
    );
    let mut command = Command::new(cargo);
    command.args(args).env(ORDER, list).current_dir(root);
    if let Some((logs, program)) = traced {
        command.env(TRACE, logs).env(TRACED, program);
    }
    match command.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("cargo {} 没通过（{status}）", args[0])),
        Err(e) => Err(format!("跑不了 cargo：{e}")),
    }
}

/// 写一个文件。
fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("写不进 {}：{e}", path.display()))
}
