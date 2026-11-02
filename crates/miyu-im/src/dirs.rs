//! im 自己的几处目录（施工 I-121，核心设计 55「数据根仿 FHS」第四节「交给程序的环境变量」）：状态目录、日志目录照核心拉起 im
//! 时给的环境变量；没给的（在进程里跑的测试、核心还没给这个名字的）照核心的门面 `DataRoot` 算同一个位置。im 不再自己拼数据根
//! 里的子路径，核心挪布局时这边只换钉。
//!
//! - 状态目录：`MIYU_STATE_DIR`，过渡期间核心一起给的旧名字 `MIYU_PACKAGE_STATE` 也认（核心 FH-6 删旧名字）。
//! - 日志目录：`MIYU_LOG_DIR`。im 自己的运行日志 `miyu-im.log` 写在这里，核心记的 im 的标准错误也在这里，叫 `stderr`。
//! - 配置目录：`MIYU_CONFIG_DIR`。主人手改的场所规则 `venues.d/`、违规词表 `moderation.txt` 在这里；旧位置（数据根下的
//!   `venues.d/`、`modules/miyu-im/`）不读，核心 FH-7 一次搬过来。
//! - 只认绝对路径：给了个相对的、空的，当没给。

use std::ffi::OsString;
use std::path::PathBuf;

use miyu_client::places::DataRoot;

use crate::PACKAGE;

/// 状态目录的名字：新名字在前。
const STATE: &[&str] = &["MIYU_STATE_DIR", "MIYU_PACKAGE_STATE"];

/// 日志目录的名字。
const LOG: &[&str] = &["MIYU_LOG_DIR"];

/// 配置目录的名字。
const CONFIG: &[&str] = &["MIYU_CONFIG_DIR"];

/// 核心记的标准错误在日志目录里叫什么（设计 55 第二节 `var/log/<编号>/stderr`）。
const STDERR: &str = "stderr";

/// 数据根 `root` 里 im 的状态目录：状态文件、卡片、聊天记录、每个驱动自己的目录都在这里。
pub fn state(root: &DataRoot) -> PathBuf {
    given(STATE, env).unwrap_or_else(|| root.package_state(PACKAGE))
}

/// 数据根 `root` 里 im 的日志目录。
pub fn logs(root: &DataRoot) -> PathBuf {
    given(LOG, env).unwrap_or_else(|| root.package_logs(PACKAGE))
}

/// 数据根 `root` 里 im 的配置目录：主人手改的场所规则、违规词表在这里。
pub fn config(root: &DataRoot) -> PathBuf {
    given(CONFIG, env).unwrap_or_else(|| root.package_config(PACKAGE))
}

/// im 自己的运行日志（[`logs`] 下的 `miyu-im.log`，`miyu_client::log::install` 照目录和名字写）。
pub fn log_file(root: &DataRoot) -> PathBuf {
    logs(root).join(format!("{PACKAGE}.log"))
}

/// 核心记的 im 的标准错误（崩的时候说的在这里）。
pub fn stderr(root: &DataRoot) -> PathBuf {
    match given(LOG, env) {
        Some(dir) => dir.join(STDERR),
        None => root.extension_stderr(PACKAGE),
    }
}

/// 照 `names` 的先后找第一个给了的绝对路径；`lookup` 读一个变量。
fn given(names: &[&str], lookup: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    names
        .iter()
        .filter_map(|name| lookup(name))
        .map(PathBuf::from)
        .find(|path| path.is_absolute())
}

/// 读这个进程的环境变量。
fn env(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

#[cfg(test)]
mod tests;
