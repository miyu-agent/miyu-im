//! 实时日志的一行说成话（设计 10，施工 I-160 上）：命令行照桥的字表（`data/human/*.json` 的 `watch/…`）说，语言跟命令行
//! 的字表。网页照 `texts.js` 的同名说法另说一份（施工 I-161）。

use serde::Serialize;

use super::{Block, Extra, Item, Judged, Line, Score};
use crate::texts::Texts;

/// 说成话的一行：本机的钟点、哪一种（界面照它配色）、字；写着的块带编号，写完了没有。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Shown {
    /// `HH:MM:SS`，本机的钟。
    pub at: String,
    /// 哪一种：`heard`、`decided`、`started`、`thinking`、`said`、`call`、`append`、`result`、`delivered`、`failed`、
    /// `recalled`、`compacted`、`ended`。
    pub kind: &'static str,
    /// 字。接上的（`append`）只有接上的那一段。
    pub text: String,
    /// 块的编号：思考、正文、调工具、接上的有。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// 块写完了：写着的是假。
    pub done: bool,
}

/// 照编号说一句、换进字段。
type Say<'a> = dyn Fn(&str, &[(&str, String)]) -> String + 'a;

/// 本机的钟点：时刻 `at`（毫秒）加时区偏移 `offset`（分钟）。
#[must_use]
pub fn clock(at: i64, offset: i32) -> String {
    let local = at + i64::from(offset) * 60_000;
    let seconds = local.rem_euclid(86_400_000) / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

/// `line` 说成话：时区偏移 `offset`（分钟）；`tool` 照工具名找显示名（汇总的那一头照结果记着），找不到的写工具名。
pub fn shown(
    line: &Line,
    offset: i32,
    texts: &Texts,
    tool: &dyn Fn(&str) -> Option<String>,
) -> Shown {
    let say = |key: &str, fields: &[(&str, String)]| texts.say(&format!("watch/{key}"), fields);
    let word = |group: &str, value: &str| {
        texts
            .try_say(&format!("watch/{group}-{value}"))
            .unwrap_or_else(|| value.to_string())
    };
    let mut id = None;
    let mut done = true;
    let (kind, text) = match &line.item {
        Item::Heard { who, text, extras } => {
            let mut parts: Vec<String> =
                extras.iter().map(|extra| extra_text(extra, &say)).collect();
            if !text.is_empty() {
                parts.insert(0, text.clone());
            }
            (
                "heard",
                say("heard", &[("who", who.clone()), ("text", parts.join(" "))]),
            )
        }
        Item::Decided {
            who,
            outcome,
            conditions,
            score,
            judge,
            why,
        } => {
            let mut parts = vec![word("outcome", outcome)];
            match conditions.is_empty() {
                true => parts.push(say("no-condition", &[])),
                false => parts.push(
                    conditions
                        .iter()
                        .map(|one| word("condition", one))
                        .collect::<Vec<_>>()
                        .join(say("separator", &[]).as_str()),
                ),
            }
            if let Some(Score { total, threshold }) = score {
                parts.push(say(
                    "score",
                    &[
                        ("total", format!("{total:.2}")),
                        ("threshold", format!("{threshold:.2}")),
                    ],
                ));
            }
            if let Some(judge) = judge {
                parts.push(judged(judge, &say, &word));
            }
            if let Some(why) = why {
                parts.push(word("why", why));
            }
            let who = who.clone().unwrap_or_default();
            (
                "decided",
                say("decided", &[("who", who), ("text", parts.join(" · "))]),
            )
        }
        Item::Started { who, glance } => match (glance, who) {
            (true, _) => ("started", say("started-glance", &[])),
            (false, Some(who)) => ("started", say("started", &[("who", who.clone())])),
            (false, None) => ("started", say("started-anyone", &[])),
        },
        Item::Wrote {
            id: block_id,
            block,
            text,
            done: finished,
        } => {
            id = Some(block_id.clone());
            done = *finished;
            match block {
                Block::Thinking => ("thinking", say("thinking", &[("text", text.clone())])),
                Block::Text => ("said", say("said", &[("text", text.clone())])),
                Block::Call(name) => {
                    let shown = tool(name).unwrap_or_else(|| name.clone());
                    (
                        "call",
                        say("call", &[("tool", shown), ("args", text.clone())]),
                    )
                }
            }
        }
        Item::Appended { id: block_id, text } => {
            id = Some(block_id.clone());
            done = false;
            ("append", text.clone())
        }
        Item::Result { tool, ok, summary } => {
            let key = if *ok { "result" } else { "result-error" };
            (
                "result",
                say(key, &[("tool", tool.clone()), ("summary", summary.clone())]),
            )
        }
        Item::Delivered { text, images } => match (text.is_empty(), images) {
            (true, 0) => ("delivered", say("delivered", &[("text", String::new())])),
            (true, count) => (
                "delivered",
                say("delivered-images", &[("count", count.to_string())]),
            ),
            (false, _) => ("delivered", say("delivered", &[("text", text.clone())])),
        },
        Item::Failed { why } => ("failed", say("failed", &[("why", word("failed-why", why))])),
        Item::Recalled { who: by } => ("recalled", say("recalled", &[("who", by.clone())])),
        Item::Compacted => ("compacted", say("compacted", &[])),
        Item::Ended { reason } => (
            "ended",
            say("ended", &[("reason", word("ended-why", reason))]),
        ),
    };
    Shown {
        at: clock(line.at, offset),
        kind,
        text,
        id,
        done,
    }
}

/// 带的东西：几张图、表情、合并转发几条、别的照种类。
fn extra_text(extra: &Extra, say: &Say<'_>) -> String {
    match extra {
        Extra::Images(count) => say("images", &[("count", count.to_string())]),
        Extra::Sticker => say("sticker", &[]),
        Extra::Forward(count) => say("forward", &[("count", count.to_string())]),
        Extra::Other(kind) => say("other", &[("kind", kind.clone())]),
    }
}

/// 判官那一段：判成了的写用时和理由，没判成的写为什么。
fn judged(judge: &Judged, say: &Say<'_>, word: &dyn Fn(&str, &str) -> String) -> String {
    let seconds = judge.millis.map_or_else(String::new, |millis| {
        format!("{:.1}", millis as f64 / 1000.0)
    });
    match (&judge.unjudged, &judge.reason) {
        (Some(why), _) if why == "timeout" => say("judge-timeout", &[("seconds", seconds)]),
        (Some(why), _) => say("judge-not", &[("why", word("why", why))]),
        (None, Some(reason)) => say("judge", &[("seconds", seconds), ("reason", reason.clone())]),
        (None, None) => say("judge", &[("seconds", seconds), ("reason", String::new())]),
    }
}
