//! `miyu im records import <文件>`（施工 I-127，设计 04 第四节）：把一份中立写法的 JSON Lines 导进 im 的聊天记录库
//! （`records/import.rs`）。照系统的语言说。
//!
//! 1. 文件读不了的说一句，退出码 1。
//! 2. 有不对的行：整份不导，标准错误上说一共几行不对，再一行一条（第几行、哪里不对，最多 `SHOWN` 条），退出码 1。
//! 3. 导成了：标准输出上说导了几条消息、几件群里的事、几条判断；有跳过的另说一句（库里已经有的、编号被别的消息占了的）。
//!    退出码 0。库写不进的说原话，退出码 1。
//!
//! 桥开着也能导（库是 WAL，等它让出锁最多几秒）；大份的照施工单先停桥再导，免得桥那几秒写不进。

use std::io::{BufReader, Write};
use std::path::Path;

use miyu_client::places::DataRoot;

use crate::records::import::{Imported, Problem, Rejected, import};
use crate::texts::Texts;

/// 读不了、有不对的行、写不进。
const FAILED: u8 = 1;

/// 不对的行最多印几条。
const SHOWN: usize = 20;

/// 导 `file` 进数据根 `root` 里 im 的库（模块的说明），交回退出码。
pub fn run(
    root: &DataRoot,
    file: &Path,
    texts: &Texts,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let path = file.display().to_string();
    let input = match std::fs::File::open(file) {
        Ok(input) => BufReader::new(input),
        Err(error) => {
            let said = texts.say(
                "records/unreadable",
                &[("path", path), ("why", error.to_string())],
            );
            return told(err, &said, FAILED);
        }
    };
    let db = crate::dirs::state(root).join(crate::records::FILE);
    let result =
        crate::records::connection(&db).and_then(|mut connection| import(&mut connection, input));
    match result {
        Ok(Ok(imported)) => told(out, &summary(texts, &path, &imported), 0),
        Ok(Err(Rejected(problems))) => {
            let mut said = vec![texts.say(
                "records/rejected",
                &[("count", problems.len().to_string()), ("path", path)],
            )];
            for (line, problem) in problems.iter().take(SHOWN) {
                said.push(texts.say(
                    "records/problem",
                    &[("line", line.to_string()), ("what", what(texts, problem))],
                ));
            }
            if problems.len() > SHOWN {
                said.push(texts.say(
                    "records/more",
                    &[("count", (problems.len() - SHOWN).to_string())],
                ));
            }
            told(err, &said.join("\n"), FAILED)
        }
        Err(error) => {
            let said = texts.say("records/not-written", &[("why", error.to_string())]);
            told(err, &said, FAILED)
        }
    }
}

/// 导成了说的：导了几样，有跳过的另一行。
fn summary(texts: &Texts, path: &str, imported: &Imported) -> String {
    let mut said = texts.say(
        "records/imported",
        &[
            ("path", path.to_string()),
            ("messages", imported.messages.to_string()),
            ("happenings", imported.happenings.to_string()),
            ("judgments", imported.judgments.to_string()),
        ],
    );
    if imported.duplicates + imported.conflicts > 0 {
        said.push('\n');
        said.push_str(&texts.say(
            "records/skipped",
            &[
                ("duplicates", imported.duplicates.to_string()),
                ("conflicts", imported.conflicts.to_string()),
            ],
        ));
    }
    said
}

/// 一行哪里不对，照给人看的字说。
fn what(texts: &Texts, problem: &Problem) -> String {
    match problem {
        Problem::Json(why) => texts.say("records/problem-json", &[("why", why.clone())]),
        Problem::Venue(venue) => texts.say("records/problem-venue", &[("venue", venue.clone())]),
        Problem::Kind(kind) => texts.say("records/problem-kind", &[("kind", kind.clone())]),
        Problem::Outcome(outcome) => {
            texts.say("records/problem-outcome", &[("outcome", outcome.clone())])
        }
        Problem::Empty(field) => {
            texts.say("records/problem-empty", &[("field", (*field).to_string())])
        }
    }
}

/// 说一句（一行或几行），交回 `code`；写不出去的是 [`FAILED`]（同 `venue show`）。
fn told(to: &mut dyn Write, said: &str, code: u8) -> u8 {
    match writeln!(to, "{said}").and_then(|()| to.flush()) {
        Ok(()) => code,
        Err(_) => FAILED,
    }
}
