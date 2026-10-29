//! 首页最近的运行日志（设计 01 第八节第 2 条：只读，能按级别筛；施工 I-6 下）：读 `state/logs/miyu-im.log` 的末尾（最多 [`TAIL`]
//! 字节），照核心的写法读每一行（`时刻 级别 来源 [会话编号] 这件事 键值`，`miyu-log` 的 `line.rs`），照级别筛，新的在前，最多
//! `limit` 行（[`MOST`] 以内）。读不了的、读不懂的行不要。页面照 `changed` 再问（日志跟着桥手里的事走，不另外盯文件）。
//! 首页选了连接的（施工 I-85 下，`scope`）：带场所（`venue=`）的行只要那个连接的，不带场所的（起来、出错这些）照旧都要。

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde_json::{Value, json};

use super::scope::{Scope, within};

/// 读文件末尾多少字节。
const TAIL: u64 = 64 * 1024;

/// 最多交几行。
pub(super) const MOST: usize = 200;

/// 筛到什么级别：`all` 全要，`warn` 要 WARN、ERROR，`error` 只要 ERROR。认不出的当 `all`。
fn wanted(level: &str, filter: &str) -> bool {
    match filter {
        "warn" => matches!(level, "WARN" | "ERROR"),
        "error" => level == "ERROR",
        _ => true,
    }
}

/// 数据根下的运行日志 `file` 里最近的几行（模块的说明）：每行 `{"date", "time", "level", "text"}`。
pub(super) fn recent(file: &Path, filter: &str, limit: usize, scope: Option<&Scope>) -> Vec<Value> {
    let Some(text) = tail(file) else {
        return Vec::new();
    };
    text.lines()
        .rev()
        .filter_map(parse)
        .filter(|line| wanted(line.1, filter))
        .filter(|line| venue(line.2).is_none_or(|venue| within(scope, venue)))
        .take(limit.min(MOST))
        .map(|(at, level, text)| {
            let (date, time) = at.split_once(' ').unwrap_or(("", at));
            let time = time.split('.').next().unwrap_or(time);
            json!({"date": date, "time": time, "level": level, "text": text})
        })
        .collect()
}

/// 文件的末尾 [`TAIL`] 字节，从下一行的头起（切在一行中间的那一段不要）。读不了的是空的。
fn tail(file: &Path) -> Option<String> {
    let mut opened = std::fs::File::open(file).ok()?;
    let length = opened.metadata().ok()?.len();
    let from = length.saturating_sub(TAIL);
    opened.seek(SeekFrom::Start(from)).ok()?;
    let mut bytes = Vec::new();
    opened.read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    if from == 0 {
        return Some(text);
    }
    Some(
        text.split_once('\n')
            .map_or(String::new(), |(_, rest)| rest.to_string()),
    )
}

/// 一行（会话编号以后的那一段）`text` 带的场所：`venue=` 的值，带引号的去掉引号。没带的是空的。
fn venue(text: &str) -> Option<&str> {
    let at = text.find(" venue=")? + " venue=".len();
    let rest = &text[at..];
    match rest.strip_prefix('"') {
        Some(quoted) => quoted.split('"').next(),
        None => rest.split(' ').next(),
    }
}

/// 一行：交回（时刻、级别、会话编号以后的那一段）。读不懂的是空的。
pub(super) fn parse(line: &str) -> Option<(&str, &str, &str)> {
    let at = line.get(..23)?;
    let rest = line.get(23..)?.trim_start();
    let (level, rest) = rest.split_once(' ')?;
    if !matches!(level, "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR") {
        return None;
    }
    let (_source, rest) = rest.trim_start().split_once(' ')?;
    let rest = rest.trim_start();
    let text = match rest.split_once(' ') {
        Some((session, after)) if session.len() == 36 && session.matches('-').count() == 4 => after,
        _ => rest,
    };
    (!text.is_empty()).then_some((at, level, text))
}
