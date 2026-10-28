//! 这一轮不说话了（施工 O-26，`onebot.md` 第一条「提供者和不说话」第 3 条）：纯逻辑。会话推来她的一条回复（`message.assistant`），
//! 里面有 `skip_reply` 的工具调用块的，这个会话的这一回合就算不说话了：这一条的字、这一轮以后她说的都不发。照回复里的调用块认，
//! 不等 `tool.call`（施工单「要定的」第 1 条）：同一条回复里先说的字，桥收到时工具还没执行。
//!
//! 施工 O-38 起桥答 `skip_reply` 带 `end_turn`，平常这一轮到这一条就结束了；调用没成的（桥停着、出错）核心照旧接着请求，
//! 以后说的还得吞，所以照旧记到 `turn.ended`（施工时定的第 240 条）。核心接着开的下一轮回合编号是新的，不受影响。
//!
//! 只记在内存里（「施工时定的」第 142 条）：（会话，回合编号）的一张表，推来那一轮的 `turn.ended` 清掉。群里桥重启照日志从头补，
//! 那一轮还在跑的照样认得出；私聊不补从前的。事件照原样的 JSON 看，群里、私聊一个看法。
//!
//! 仅发送最终消息（施工 O-57，`speak.rs` 的 `say_or_hold`）也记在这里：她带工具调用的回复里的字先扣着（[`Quiet::hold`]），同
//! 一回合后面说了不带工具调用的话、回合结束了再交出来（[`Quiet::release`]）；这一轮不说话了的，扣着的丢掉。

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::voicing::Voice;

/// 这一轮不说话了的会话和回合；仅发送最终消息的场所里扣着的那段话。
#[derive(Default)]
pub(super) struct Quiet {
    /// （会话编号，回合编号）。
    turns: HashSet<(String, u64)>,
    /// 扣着的：（她说话的那条线，回合编号）到那段话。
    held: HashMap<(String, u64), Held>,
    /// 「这一轮不说话」那件工具叫什么（施工 T-4：驱动交的，`ToolSet::silence`）：一个驱动一件（施工 I-70 中上），没有的是空的。
    silence: Vec<&'static str>,
}

/// 扣着的一段话：发回哪个会话、哪一种、字。
pub(super) struct Held {
    pub(super) session: String,
    pub(super) voice: Voice,
    pub(super) text: String,
}

impl Quiet {
    /// 会话 `session` 推来的一条事件 `event`：她的回复里有 `skip_reply` 的调用块的，记下这一轮；这一轮的 `turn.ended` 清掉。交回
    /// 这一条是不是她这一轮不说话了以后的回复：是的，里面的字不发。没有回合编号的不看。
    pub(super) fn heard(&mut self, session: &str, event: &Value) -> bool {
        let Some(turn) = event["turn"].as_u64() else {
            return false;
        };
        let key = (session.to_string(), turn);
        match event["kind"].as_str() {
            Some("message.assistant") => {
                if skips(event, &self.silence) {
                    // 这一轮不说话了：仅发送最终消息时扣着的也不发。
                    self.held.remove(&key);
                    self.turns.insert(key.clone());
                }
                self.turns.contains(&key)
            }
            Some("turn.ended") => {
                self.turns.remove(&key);
                false
            }
            _ => false,
        }
    }
}

impl Quiet {
    /// 还什么都没记的一份：「这一轮不说话」的工具叫 `silence` 里的名字（施工 T-4；施工 I-70 中上起每个驱动一件）。
    pub(super) fn new(silence: impl IntoIterator<Item = &'static str>) -> Quiet {
        Quiet {
            silence: silence.into_iter().collect(),
            ..Quiet::default()
        }
    }

    /// 扣着线 `line` 第 `turn` 回合的一段话（仅发送最终消息）：同一回合扣过的换成它。
    pub(super) fn hold(&mut self, line: &str, turn: u64, held: Held) {
        self.held.insert((line.to_string(), turn), held);
    }

    /// 交出线 `line` 第 `turn` 回合扣着的那段话（没有的是空的），不再扣着。
    pub(super) fn release(&mut self, line: &str, turn: u64) -> Option<Held> {
        self.held.remove(&(line.to_string(), turn))
    }
}

/// 回复 `event` 里有没有工具调用块（仅发送最终消息：有的是中间那几步）。
pub(super) fn has_calls(event: &Value) -> bool {
    event["body"]["blocks"]
        .as_array()
        .is_some_and(|blocks| blocks.iter().any(|block| block["type"] == "tool_call"))
}

/// 回复 `event` 里有没有「这一轮不说话」的工具（`silence` 里的一件）的调用块。
fn skips(event: &Value, silence: &[&str]) -> bool {
    event["body"]["blocks"].as_array().is_some_and(|blocks| {
        blocks.iter().any(|block| {
            block["type"] == "tool_call"
                && block["name"]
                    .as_str()
                    .is_some_and(|name| silence.contains(&name))
        })
    })
}

#[cfg(test)]
mod tests;
