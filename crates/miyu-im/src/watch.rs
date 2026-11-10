//! 实时日志的纯逻辑（设计 10，施工 I-160 上）：场所会话的一条事件（原样的 JSON）整理成零到几行，给人看她在干嘛。
//!
//! - 只认这几种：收到的消息（`message.user`）、判断（`ext.im.chat.decided`）、开轮（`turn.started`）、她写的思考、正文、调的工具
//!   （`message.assistant`，写的时候是 `model.delta`）、工具的结果（`tool.result`）、发出去的（`venue.delivered`）、没发出去的
//!   （`ext.im.venues.failed`）、撤回（`venue.recalled`）、压缩（`context.compacted`）、没正常收尾的一轮（`turn.ended`）。别的不出行。
//! - 行是结构化的，不带说法：命令行照桥的字表在 [`render`] 说成话，网页照 `texts.js` 说，各跟各的界面语言。汇总的那一头
//!   （[`hub`]）照原样推给看着的。
//! - 思考、正文、调工具照块出行：同一块的编号是 `<线>:<seen>:<第几块>`，写的时候的增量和写完的那一条对得上，界面照编号接上、换掉。
//! - 谁发的照场所的格（`venue.name`）；开轮、判断照触发的那一条的序号问 `names`（汇总的那一头记着最近的几条）。
//! - 不碰 I/O、时钟：时刻原样带着，换成本机的钟点在 [`render`]。

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod follow;
pub mod hub;
pub mod render;

/// 一行：什么时候、这一行是什么。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Line {
    /// 事件的时刻，毫秒。
    pub at: i64,
    /// 这一行是什么。
    pub item: Item,
}

/// 收到的消息里带的东西。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Extra {
    /// 几张图。
    Images(usize),
    /// 表情。
    Sticker,
    /// 合并转发，里面几条。
    Forward(usize),
    /// 别的（文件、语音、视频……）：种类原样。
    Other(String),
}

/// 判断的分数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Score {
    /// 总分。
    pub total: f64,
    /// 门槛。
    pub threshold: f64,
}

/// 判官怎么说的。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Judged {
    /// 理由；没判成的是空的。
    pub reason: Option<String>,
    /// 用了多久，毫秒。
    pub millis: Option<u64>,
    /// 没判成的原因（`timeout`、`rate_full` 这些），判成了的是空的。
    pub unjudged: Option<String>,
}

/// 一块：思考、正文、调工具。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "tool", rename_all = "snake_case")]
pub enum Block {
    /// 思考。
    Thinking,
    /// 正文。
    Text,
    /// 调工具：工具名。
    Call(String),
}

/// 一行是什么。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Item {
    /// 收到一条：谁、说了什么、带了什么、是不是叫了她。
    Heard {
        /// 谁。
        who: String,
        /// 字。
        text: String,
        /// 带的东西。
        extras: Vec<Extra>,
    },
    /// 判断：谁的、结局（`reply`、`record`……）、走哪条路、成立的条件、分数、判官、只记下的原因。
    Decided {
        /// 触发的那一条是谁的。
        who: Option<String>,
        /// 结局。
        outcome: String,
        /// 成立的条件（`direct`、`after_speaking`……）。
        conditions: Vec<String>,
        /// 分数。
        score: Option<Score>,
        /// 判官。
        judge: Option<Judged>,
        /// 进站链挡下的原因（`rate_limited` 这些）。
        why: Option<String>,
    },
    /// 开了一轮：谁触发的；看一眼开的。
    Started {
        /// 触发的那一条是谁的。
        who: Option<String>,
        /// 时不时看一眼开的。
        glance: bool,
    },
    /// 一块：编号、是什么、到目前的字、写完了没有。
    Wrote {
        /// `<线>:<seen>:<第几块>`。
        id: String,
        /// 是什么。
        block: Block,
        /// 到目前的字（增量接上来的）。
        text: String,
        /// 写完了。
        done: bool,
    },
    /// 写着的一块又接上一段字：编号、接上的字。界面照编号接到那一行后面。
    Appended {
        /// `<线>:<seen>:<第几块>`。
        id: String,
        /// 接上的字。
        text: String,
    },
    /// 工具的结果：显示名（没有的是工具名）、成没成、摘要。
    Result {
        /// 显示名。
        tool: String,
        /// 成了。
        ok: bool,
        /// 摘要。
        summary: String,
    },
    /// 发出去了：字、几张图。
    Delivered {
        /// 字。
        text: String,
        /// 几张图。
        images: usize,
    },
    /// 没发出去。
    Failed {
        /// 为什么（`expired` 这些）。
        why: String,
    },
    /// 有人撤回了一条。
    Recalled {
        /// 谁（平台上的身份）。
        who: String,
    },
    /// 压缩了上下文。
    Compacted,
    /// 一次模型请求出错了（施工 I-200）：哪个模型、端点说的原话。这一轮多半跟着停（[`Item::Ended`]），重试成了的接着走。
    Errored {
        /// 模型。
        model: String,
        /// 原话，最多 [`ERROR_CHARS`] 个字。
        message: String,
    },
    /// 这一轮没正常收尾。
    Ended {
        /// 原因（`interrupted` 这些）。
        reason: String,
    },
}

/// 工具结果的摘要最多几个字。
pub const SUMMARY_CHARS: usize = 120;

/// 出错的原话最多几个字。
pub const ERROR_CHARS: usize = 300;

/// 一条事件 `event` 整理成的行。`lane` 是它在哪条线（会话编号），块的编号带上它；`names` 照序号找那一条是谁说的。
pub fn lines(event: &Value, lane: &str, names: &dyn Fn(u64) -> Option<String>) -> Vec<Line> {
    let at = event["at"]
        .as_str()
        .and_then(|at| miyu_client::time::Timestamp::parse(at).ok())
        .map_or(0, miyu_client::time::Timestamp::unix_millis);
    let body = &event["body"];
    let items = match event["kind"].as_str().unwrap_or_default() {
        "message.user" => heard(body).into_iter().collect(),
        "ext.im.chat.decided" => vec![decided(body, names)],
        "turn.started" => vec![Item::Started {
            who: body["trigger"].as_u64().and_then(names),
            glance: event["cause"]
                .as_str()
                .is_some_and(|cause| cause.starts_with("glance/")),
        }],
        "message.assistant" => wrote(body, lane),
        "model.delta" => delta(body, lane).into_iter().collect(),
        "tool.result" => vec![result(body)],
        "venue.delivered" => vec![Item::Delivered {
            text: text_of(&body["text"]),
            images: body["images"].as_array().map_or(0, Vec::len),
        }],
        "ext.im.venues.failed" => vec![Item::Failed {
            why: text_of(&body["why"]),
        }],
        "venue.recalled" => vec![Item::Recalled {
            who: text_of(&body["by"]),
        }],
        "context.compacted" => vec![Item::Compacted],
        // 模型请求出错的原因（施工 I-200，项目主人 11-10：出错了只看到「这一轮停了：出错」）。
        "model.called" if body["result"] == "error" => vec![Item::Errored {
            model: text_of(&body["model"]),
            message: cut(&text_of(&body["error"]["message"]), ERROR_CHARS),
        }],
        "turn.ended" => match body["reason"].as_str() {
            Some("completed") | None => Vec::new(),
            Some(reason) => vec![Item::Ended {
                reason: reason.to_string(),
            }],
        },
        _ => Vec::new(),
    };
    items.into_iter().map(|item| Line { at, item }).collect()
}

/// 收到的一条。只有占位、什么都没带的（照说不会）不出行。
fn heard(body: &Value) -> Option<Item> {
    let venue = &body["venue"];
    let text = blocks_text(&body["blocks"]);
    let mut extras = Vec::new();
    let media = venue["media"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let images = media.iter().filter(|one| one["kind"] == "image").count();
    if images > 0 {
        extras.push(Extra::Images(images));
    }
    for one in media {
        match one["kind"].as_str() {
            Some("image") => {}
            Some("sticker") => extras.push(Extra::Sticker),
            Some(other) => extras.push(Extra::Other(other.to_string())),
            None => {}
        }
    }
    for forward in venue["forwards"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
    {
        extras.push(Extra::Forward(
            forward["nodes"].as_array().map_or(0, Vec::len),
        ));
    }
    // 只带东西的，正文是占位（`[forward]` 这些）：东西另写了，占位不再重复。
    let text = if !extras.is_empty() && text.starts_with('[') && text.ends_with(']') {
        String::new()
    } else {
        text
    };
    if text.is_empty() && extras.is_empty() {
        return None;
    }
    let who = venue["name"]
        .as_str()
        .map_or_else(|| text_of(&body["from"]), str::to_string);
    Some(Item::Heard { who, text, extras })
}

/// 判断的一行。
fn decided(body: &Value, names: &dyn Fn(u64) -> Option<String>) -> Item {
    let who = body["msgs"]
        .as_array()
        .and_then(|msgs| msgs.last())
        .and_then(Value::as_u64)
        .and_then(names);
    let conditions = body["conditions"]
        .as_array()
        .map(|all| {
            all.iter()
                .filter_map(|one| one["kind"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let score = match (
        body["score"]["total"].as_f64(),
        body["score"]["threshold"].as_f64(),
    ) {
        (Some(total), Some(threshold)) => Some(Score { total, threshold }),
        _ => None,
    };
    let judge = body
        .get("judge")
        .filter(|judge| judge.is_object())
        .map(|judge| Judged {
            reason: judge["answer"]["reason"].as_str().map(str::to_string),
            millis: judge["millis"].as_u64(),
            unjudged: judge["unjudged"].as_str().map(str::to_string),
        });
    Item::Decided {
        who,
        outcome: text_of(&body["outcome"]),
        conditions,
        score,
        judge,
        why: body["why"].as_str().map(str::to_string),
    }
}

/// 写完的一条：每块一行，写完了。
fn wrote(body: &Value, lane: &str) -> Vec<Item> {
    let seen = body["seen"].as_u64().unwrap_or_default();
    let blocks = body["blocks"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    blocks
        .iter()
        .enumerate()
        .filter_map(|(index, one)| {
            let (block, text) = match one["type"].as_str()? {
                "reasoning" => (Block::Thinking, text_of(&one["text"])),
                "text" => (Block::Text, text_of(&one["text"])),
                "tool_call" => (Block::Call(text_of(&one["name"])), text_of(&one["args"])),
                _ => return None,
            };
            let text = text.trim().to_string();
            (!text.is_empty() || matches!(block, Block::Call(_))).then(|| Item::Wrote {
                id: format!("{lane}:{seen}:{index}"),
                block,
                text,
                done: true,
            })
        })
        .collect()
}

/// 写着的一段增量：开头一块（带是什么）、接上的字。写完那一段不出行：写完的那一条（`message.assistant`）照同一个编号定下来。
fn delta(body: &Value, lane: &str) -> Option<Item> {
    let id = format!(
        "{lane}:{}:{}",
        body["seen"].as_u64().unwrap_or_default(),
        body["index"].as_u64().unwrap_or_default()
    );
    let block = match body["start"].as_str() {
        Some("reasoning" | "thinking") => Some(Block::Thinking),
        Some("text") => Some(Block::Text),
        Some("tool_call") => Some(Block::Call(text_of(&body["name"]))),
        _ => None,
    };
    match (block, body["text"].as_str()) {
        (Some(block), _) => Some(Item::Wrote {
            id,
            block,
            text: String::new(),
            done: false,
        }),
        (None, Some(text)) => Some(Item::Appended {
            id,
            text: text.to_string(),
        }),
        _ => None,
    }
}

/// 工具的结果。
fn result(body: &Value) -> Item {
    let ok = body["status"] == "ok";
    let tool = ["zh", "en"]
        .iter()
        .find_map(|lang| body["display_name"][lang].as_str())
        .map(str::to_string)
        .unwrap_or_default();
    let blocks = body["blocks"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut parts = Vec::new();
    let text = blocks_text(&body["blocks"]);
    if !text.is_empty() {
        let most = if ok { SUMMARY_CHARS } else { ERROR_CHARS };
        parts.push(cut(&first_lines(&text, ok), most));
    }
    for image in blocks.iter().filter(|one| one["type"] == "image") {
        match (image["width"].as_u64(), image["height"].as_u64()) {
            (Some(width), Some(height)) => parts.push(format!("{width}×{height}")),
            _ => parts.push(String::new()),
        }
    }
    Item::Result {
        tool,
        ok,
        summary: parts.join(" · "),
    }
}

/// 几块里的字连起来。
fn blocks_text(blocks: &Value) -> String {
    blocks
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|one| one["type"] == "text")
        .filter_map(|one| one["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// 成了的只留第一行（后面还有几行写进去）；出错的整段，换行换成空格。
fn first_lines(text: &str, ok: bool) -> String {
    if !ok {
        return text.split_whitespace().collect::<Vec<_>>().join(" ");
    }
    let mut rows = text.lines().filter(|row| !row.trim().is_empty());
    let first = rows.next().unwrap_or_default().trim().to_string();
    match rows.count() {
        0 => first,
        more => format!("{first} (+{more})"),
    }
}

/// 截到 `most` 个字，截了的接一个「…」。
fn cut(text: &str, most: usize) -> String {
    match text.char_indices().nth(most) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text.to_string(),
    }
}

/// 字的格：字符串照原样，别的空着。
fn text_of(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests;
