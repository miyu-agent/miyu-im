//! 登记簿对账（照核心仓库的 `xtask/src/ledger.rs`，核心设计 26 第十节 J12、设计 32 第九节第 3 条；这个仓库施工 S-6 中）：
//! 包文件夹 `package/data/` 里给模型看的每一份字都登记在 [`PATH`] 的那张表里。
//!
//! 门禁查两边一一对上：`package/data/` 下每一份给模型看的文件都在表里，表里每一行都有文件，指纹对得上。指纹是文件字节的
//! SHA-256 的前 8 位：字改了就对不上，逼着回登记簿重新量 token、写上为什么改。算在「文档」那一项里。

use std::collections::BTreeMap;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::drawing;

/// 登记簿在仓库里的位置。
pub const PATH: &str = "docs/ledger.md";

const SECTION: &str = "登记簿";
const HEADER: [&str; 6] = [
    "文件",
    "进到哪",
    "什么时候加进来",
    "token",
    "为什么加",
    "指纹",
];

/// 给模型看的字都在这个目录里（相对仓库根）：判官的说明、平台工具的说明、事实、工具结果。
pub(crate) const DATA: &str = "package/data";

/// 给人看的字放在这样的目录里，不登记（核心施工 4-5 上）。
const HUMAN: &str = "human";

/// `data/` 里这几份是数据，不发给模型，不登记（核心施工 S-6 上从 `software/onebot/` 挪来时的豁免，原样搬过来）：群聊内核的
/// 出厂参数（`chat.md` 第八条施工时定的第 16 条）、违规词表（只拿来比子串）、出厂的场所规则（第八条施工时定的第 21 条）、桥
/// 自己的数（路径、等回应多久、队列多长，`onebot.md`「施工时定的」第 15 条）。只豁免这几份：`venues.d/` 里别的文件照查。
const EXEMPT: [&str; 4] = [
    "package/data/bridge.json",
    "package/data/defaults.toml",
    "package/data/moderation.txt",
    "package/data/venues.d/50-defaults.toml",
];

/// 查一遍，交回对不上的地方。
pub fn check(root: &Path) -> Vec<String> {
    let text = match std::fs::read_to_string(root.join(PATH)) {
        Ok(text) => text,
        Err(e) => return vec![format!("读不了登记簿 {PATH}：{e}")],
    };
    let rows = match rows(&text) {
        Ok(rows) => rows,
        Err(e) => return vec![format!("登记簿 {PATH} 读不出来：{e}")],
    };
    let mut files = BTreeMap::new();
    match walk(&root.join(DATA), &format!("{DATA}/"), &mut files) {
        Ok(()) => compare(&rows, &files),
        Err(e) => vec![e],
    }
}

/// 表里的每一行：文件（相对仓库根的路径）到登记的指纹。
fn rows(text: &str) -> Result<BTreeMap<String, String>, String> {
    let section = drawing::section(text, SECTION)?;
    let tables = drawing::tables(&section);
    let table = drawing::find_table(&tables, &HEADER)?;
    let mut rows = BTreeMap::new();
    for row in &table.rows {
        if row.len() != HEADER.len() {
            return Err(format!(
                "登记簿的每一行要有 {} 格：{}",
                HEADER.len(),
                row.join(" | ")
            ));
        }
        let quoted = |cell: Option<&String>| {
            cell.and_then(|cell| drawing::backticked(cell).into_iter().next())
                .ok_or_else(|| format!("这一行的文件、指纹要写在反引号里：{}", row.join(" | ")))
        };
        let file = quoted(row.first())?;
        let fingerprint = quoted(row.last())?;
        if rows.insert(file.clone(), fingerprint).is_some() {
            return Err(format!("{file} 登记了两行"));
        }
    }
    Ok(rows)
}

/// `dir` 下每一份要登记的文件：相对仓库根的路径（用 `/` 分开，三个平台一样）到字节。
fn walk(dir: &Path, prefix: &str, files: &mut BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let unreadable = |e: std::io::Error| format!("读不了 {}：{e}", dir.display());
    for entry in std::fs::read_dir(dir).map_err(unreadable)? {
        let entry = entry.map_err(unreadable)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = format!("{prefix}{name}");
        if entry.file_type().map_err(unreadable)?.is_dir() {
            if name != HUMAN {
                walk(&entry.path(), &format!("{path}/"), files)?;
            }
        } else if !EXEMPT.contains(&path.as_str()) {
            files.insert(path, std::fs::read(entry.path()).map_err(unreadable)?);
        }
    }
    Ok(())
}

/// 两边比：没登记的、登记了却没有文件的、指纹对不上的。
fn compare(rows: &BTreeMap<String, String>, files: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    let mut problems = Vec::new();
    for (file, bytes) in files {
        let actual = fingerprint(bytes);
        match rows.get(file) {
            None => problems.push(format!(
                "{file} 没登记：给模型看的字都要登记在 {PATH}（进到哪、什么时候加进来、多少 token、为什么加），指纹是 {actual}"
            )),
            Some(recorded) if *recorded != actual => problems.push(format!(
                "{file} 的字变了，登记簿里的指纹还是 {recorded}：重新量 token、写上为什么改，指纹换成 {actual}"
            )),
            Some(_) => {}
        }
    }
    for file in rows.keys().filter(|file| !files.contains_key(*file)) {
        problems.push(format!("登记簿里有 {file}，{DATA}/ 下却没有这份文件"));
    }
    problems
}

/// 指纹：字节的 SHA-256 的前 8 位十六进制。
fn fingerprint(bytes: &[u8]) -> String {
    Sha256::digest(bytes)[..4]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests;
