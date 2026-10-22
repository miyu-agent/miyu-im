//! 门禁程序（照核心仓库的 `xtask`，施工 S-6 中）：`cargo xtask check` 依次跑格式、clippy、文档、纯逻辑、行数、许可证和测试，
//! 最后打一张结果表。本机验收和三平台 CI 跑的是同一条命令。设了 `MIYU_BUILD_LOCK` 的，编译的几步照锁排队（[`BUILD_LOCK`]）。
//!
//! 「文档」那一项还查登记簿：`im/data/` 里给模型看的字和 `docs/ledger.md` 一一对上（[`ledger`]）。「纯逻辑」查群聊内核
//! 不碰 I/O、时钟，不用遍历顺序会变的容器（[`purity`]，施工 O-40）。「行数」查每个 `.rs`
//! 不超过 500 行，测试也算。「许可证」查发布的四个平台上用得到的第三方依赖，许可证都能和 GPL-3.0-or-later 合在一起发。
//! 「测试」另查每个测试文件都进了测试程序（`autotests = false` 的 crate）。里面的 cargo 一个接一个跑，不并行。

mod drawing;
mod files;
mod ledger;
mod licenses;
mod purity;
mod size;
mod test_targets;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// 放 Rust 代码的目录（相对仓库根）：`crates/` 下每个 crate，加上门禁自己。
const CODE: [&str; 2] = ["crates", "xtask"];

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("check") => check(),
        _ => {
            eprintln!("用法：cargo xtask check");
            ExitCode::from(2)
        }
    }
}

/// 一项检查的结果。`problems` 是空的就算通过。
struct Outcome {
    name: &'static str,
    problems: Vec<String>,
}

/// 仓库的根目录。
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask 在仓库根目录下的 xtask/ 里，一定有上一级目录")
        .to_path_buf()
}

fn check() -> ExitCode {
    let root = root();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let mut outcomes = vec![
        cargo_step(
            &cargo,
            &root,
            "格式",
            &["fmt", "--all", "--check"],
            &[],
            false,
        ),
        cargo_step(
            &cargo,
            &root,
            "clippy",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--locked",
                "--quiet",
                "--",
                "-D",
                "warnings",
            ],
            &[],
            true,
        ),
        // 注释里的链接断了、写法坏了，生成文档时才看得出来，所以警告也当错。私有的也要生成。
        cargo_step(
            &cargo,
            &root,
            "文档",
            &[
                "doc",
                "--workspace",
                "--no-deps",
                "--document-private-items",
                "--locked",
                "--quiet",
            ],
            &[("RUSTDOCFLAGS", "-D warnings")],
            true,
        ),
    ];
    println!("\n── 文档：登记簿 {} 和 {}/ ──", ledger::PATH, ledger::DATA);
    if let Some(docs) = outcomes.last_mut() {
        docs.problems.extend(ledger::check(&root));
    }
    println!("\n── 纯逻辑：{} ──", purity::DIRS.join("、"));
    outcomes.push(Outcome {
        name: "纯逻辑",
        problems: pure(&root),
    });
    println!("\n── 行数：每个 .rs 不超过 {} 行 ──", size::MAX_LINES);
    outcomes.push(Outcome {
        name: "行数",
        problems: lines(&root),
    });
    println!(
        "\n── 许可证：cargo metadata，{} ──",
        licenses::PLATFORMS.join("、")
    );
    outcomes.push(Outcome {
        name: "许可证",
        problems: licenses::check(&root, &cargo),
    });
    // 先在锁里把测试程序都编好，再出锁跑（2026-10-22 项目主人定：锁只管编译）。编不过的不再跑。
    let mut tests = cargo_step(
        &cargo,
        &root,
        "测试",
        &["test", "--workspace", "--locked", "--no-run", "--quiet"],
        &[],
        true,
    );
    if tests.problems.is_empty() {
        tests = cargo_step(
            &cargo,
            &root,
            "测试",
            &["test", "--workspace", "--locked", "--quiet"],
            &[],
            false,
        );
    }
    println!("\n── 测试：每个测试文件都进了测试程序（autotests = false 的 crate）──");
    tests.problems.extend(test_crates(&root));
    outcomes.push(tests);
    report(&outcomes)
}

/// 几个会话共用一台机器时的编译锁（2026-10-22 项目主人定，同核心仓库）：设了这个环境变量（锁文件的路径）的，编译那几步照
/// `flock <锁文件>` 排队，跑测试、查格式这些不占锁。没设的照旧直接跑（CI、别的机器）。
const BUILD_LOCK: &str = "MIYU_BUILD_LOCK";

/// 跑一条 cargo 命令，输出照常打在终端上。`env` 是只加给这一条命令的环境变量，标题里照样打出来，方便照着手动重跑。
/// `compiles` 的（要编译的那几步）照 [`BUILD_LOCK`] 排队。
fn cargo_step(
    cargo: &str,
    root: &Path,
    name: &'static str,
    args: &[&str],
    env: &[(&str, &str)],
    compiles: bool,
) -> Outcome {
    let shown: String = env.iter().map(|(k, v)| format!("{k}=\"{v}\" ")).collect();
    println!("\n── {name}：{shown}cargo {} ──", args.join(" "));
    let lock = std::env::var(BUILD_LOCK)
        .ok()
        .filter(|lock| compiles && !lock.is_empty());
    let mut command = match &lock {
        Some(lock) => {
            let mut flock = Command::new("flock");
            flock.arg(lock).arg(cargo);
            flock
        }
        None => Command::new(cargo),
    };
    let problems = match command
        .args(args)
        .envs(env.iter().copied())
        .current_dir(root)
        .status()
    {
        Ok(status) if status.success() => Vec::new(),
        Ok(status) => vec![format!(
            "cargo {} 没通过（{status}），原因见上面的输出",
            args[0]
        )],
        Err(e) => vec![format!("跑不了 cargo {}：{e}", args[0])],
    };
    Outcome { name, problems }
}

/// 每个 `.rs` 文件的行数（[`size`]）。
fn lines(root: &Path) -> Vec<String> {
    sources(root, &CODE, |label, text| {
        size::check(label, text, size::MAX_LINES)
            .into_iter()
            .collect()
    })
}

/// 纯逻辑的代码里不许有的（[`purity`]）。
fn pure(root: &Path) -> Vec<String> {
    sources(root, &purity::DIRS, purity::scan)
}

/// 照 `dirs`（相对仓库根）下每个 `.rs` 文件的路径和内容查一遍 `check`，问题并在一起；读不了的记进去。
fn sources(root: &Path, dirs: &[&str], check: impl Fn(&str, &str) -> Vec<String>) -> Vec<String> {
    let mut problems = Vec::new();
    for dir in dirs {
        let found = match files::rust_files(&root.join(dir)) {
            Ok(found) => found,
            Err(e) => {
                problems.push(e);
                continue;
            }
        };
        for path in found {
            let label = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            match std::fs::read_to_string(&path) {
                Ok(text) => problems.extend(check(&label, &text)),
                Err(e) => problems.push(format!("读不了 {label}：{e}")),
            }
        }
    }
    problems
}

/// 查每个 crate 的测试文件都进了测试程序（[`test_targets`]）：`crates/` 下每个目录，加上门禁自己。
fn test_crates(root: &Path) -> Vec<String> {
    let mut dirs = vec![root.join("xtask")];
    match std::fs::read_dir(root.join("crates")) {
        Ok(entries) => dirs.extend(
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir()),
        ),
        Err(e) => return vec![format!("读不了 crates/：{e}")],
    }
    dirs.sort();
    let dirs: Vec<&Path> = dirs.iter().map(PathBuf::as_path).collect();
    test_targets::check(root, &dirs)
}

fn report(outcomes: &[Outcome]) -> ExitCode {
    println!("\n── 门禁结果 ──");
    for outcome in outcomes {
        if outcome.problems.is_empty() {
            println!("  ✓ {}", outcome.name);
        } else {
            println!("  ✗ {}：{} 处", outcome.name, outcome.problems.len());
            for problem in &outcome.problems {
                println!("      {problem}");
            }
        }
    }
    if outcomes.iter().all(|o| o.problems.is_empty()) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
