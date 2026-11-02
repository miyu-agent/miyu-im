//! 账本放在哪（施工 I-121，核心设计 55 第四节「交给程序的环境变量」）：照核心拉起时给的状态目录 `MIYU_STATE_DIR`，过渡期间
//! 核心一起给的旧名字 `MIYU_PACKAGE_STATE` 也认；都没给的（或给的不是绝对路径）照工作目录，核心给的工作目录就是状态目录。

use std::ffi::OsString;
use std::path::PathBuf;

/// 状态目录的名字：新名字在前。
const STATE: &[&str] = &["MIYU_STATE_DIR", "MIYU_PACKAGE_STATE"];

/// 文件 `name` 在哪：照这个进程的环境变量。
pub fn of(name: &str) -> PathBuf {
    within(name, |key| std::env::var_os(key))
}

/// 同 [`of`]，`lookup` 读一个变量。
fn within(name: &str, lookup: impl Fn(&str) -> Option<OsString>) -> PathBuf {
    STATE
        .iter()
        .filter_map(|key| lookup(key))
        .map(PathBuf::from)
        .find(|dir| dir.is_absolute())
        .map_or_else(|| PathBuf::from(name), |dir| dir.join(name))
}

#[cfg(test)]
mod tests;
