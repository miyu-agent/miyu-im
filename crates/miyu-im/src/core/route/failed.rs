//! 没回成的一轮（施工 I-275）：她的一轮出错结束了，桥原来只摘掉贴的表情，运行日志里一行都不记，群里看着就是不理人（11-23：
//! 主会话的上下文超过窗口、自动压缩暂停以后，每一轮 30 毫秒就报错结束，一晚上没人知道）。这里照推来的事件认出这种一轮，交回
//! 要记的那一行，由 `pushed.rs` 记进运行日志。纯逻辑，事件照原样的 JSON 看。
//!
//! - 模型请求出错（`model.called` 的 `result` 是 `error`）：记下这一轮最后一次的分类和原话（核心说细节在它里面）。
//! - 这一轮结束（`turn.ended`）：走完了的（`completed`）不算；别的交回一笔 [`Failed`]，带上记下的那次出错。
//! - 只记在内存里：（会话，回合编号）一张表，那一轮结束就清。补来的旧事件由调的一方挡在外面，不交进来。

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use serde_json::Value;

/// 原话最多留几个字：核心的报错可能带一整段请求。
pub(super) const MESSAGE_CHARS: usize = 300;

/// 没回成的一轮。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Failed {
    /// 回合编号。
    pub(super) turn: u64,
    /// `turn.ended` 的原因：`error`、`step_limit`、`interrupted`……
    pub(super) reason: String,
    /// 这一轮最后一次模型请求出错的分类（`compaction_paused`、`rate_limited`……）；没出过错的是空的。
    pub(super) class: String,
    /// 那一次出错的原话，截到 [`MESSAGE_CHARS`] 个字。
    pub(super) message: String,
}

impl Failed {
    /// 是不是要提醒的：人打断的、有计划的重启接着干的不算。
    pub(super) fn alarming(&self) -> bool {
        !matches!(self.reason.as_str(), "interrupted" | "restarted")
    }
}

/// 在跑的几轮里出过的错。
#[derive(Default)]
pub(super) struct Failures {
    /// （会话编号，回合编号）到最后一次出错的（分类，原话）。
    errors: HashMap<(String, u64), (String, String)>,
}

impl Failures {
    /// 会话 `session` 推来的一条事件 `event`：模型请求出错的记下；这一轮结束了、不是走完的，交回一笔。没有回合编号的不看。
    pub(super) fn heard(&mut self, session: &str, event: &Value) -> Option<Failed> {
        let turn = event["turn"].as_u64()?;
        let key = (session.to_string(), turn);
        let body = &event["body"];
        match event["kind"].as_str()? {
            "model.called" if body["result"] == "error" => {
                let error = &body["error"];
                let class = error["class"].as_str().unwrap_or_default().to_string();
                let message = cut(error["message"].as_str().unwrap_or_default());
                self.errors.insert(key, (class, message));
                None
            }
            "turn.ended" => {
                let (class, message) = self.errors.remove(&key).unwrap_or_default();
                let reason = body["reason"].as_str().unwrap_or("completed");
                (reason != "completed").then(|| Failed {
                    turn,
                    reason: reason.to_string(),
                    class,
                    message,
                })
            }
            _ => None,
        }
    }
}

/// 截到 [`MESSAGE_CHARS`] 个字，换行换成空格（运行日志一笔一行）。
fn cut(text: &str) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    match flat.char_indices().nth(MESSAGE_CHARS) {
        Some((at, _)) => format!("{}…", &flat[..at]),
        None => flat,
    }
}
