//! 复读和「她已经在回的」（施工 I-28，`onebot.md`「群里怎么叫她」第 5、9、10 条）：投影里每条人话记一个指纹（群聊内核的
//! `echo`），判一条之前看它是不是在复读、她回的那一条后来有没有人跟着复读；问判官时交「她已经决定要回、还没回完的」几条，
//! 写着谁说的、正文的开头。正文的开头只留最近 [`KEPT`] 条，判过要回的都是刚来的。

use miyu_chat::{Answering, Spoken, echoed, repeats};
use miyu_client::events::Block;
use miyu_client::time::Timestamp;

use super::{Projection, Speaker};

/// 正文的开头最多留几条：只给判官看她刚决定要回的那几条，用不着更早的。
const KEPT: usize = 200;

/// 正文的开头留几个字符：认得出是哪一句就够。
const HEAD: usize = 60;

/// 一条人话的正文：字块接起来（图、文件这些不算）。
pub(super) fn words(blocks: &[Block]) -> String {
    blocks
        .iter()
        .filter_map(|block| match block {
            Block::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect()
}

/// 序号是 `seq` 的一条人话里复读要看的：`judging` 是此刻正在判的几条（还没记下判断）。
fn spoken(seq: u64, speaker: &Speaker, judging: &[u64]) -> Spoken {
    Spoken {
        sender: speaker.id.clone(),
        at: speaker.at,
        echo: speaker.echo,
        counted: speaker.passed || judging.contains(&seq),
    }
}

impl Projection {
    /// 记下序号是 `seq` 的人话的正文开头（只留最近 [`KEPT`] 条）。
    pub(super) fn remember(&mut self, seq: u64, text: &str) {
        let head: String = text.trim().chars().take(HEAD).collect();
        if head.is_empty() {
            return;
        }
        self.heads.insert(seq, head);
        while self.heads.len() > KEPT {
            self.heads.pop_first();
        }
    }

    /// 记下的一笔判断 `body`（`ext.im.chat.decided`）：进站链放行了的，判的那几条记成判过了（复读只看判过的）。
    pub(super) fn passed(&mut self, body: &serde_json::Value) {
        if body["inbound"] != "pass" {
            return;
        }
        for seq in body["msgs"].as_array().into_iter().flatten() {
            if let Some(speaker) = seq.as_u64().and_then(|seq| self.said.get_mut(&seq)) {
                speaker.passed = true;
            }
        }
    }

    /// 序号是 `seq` 的这一条此刻（`now`）是不是在复读：在它之前、窗口 `window` 毫秒首尾相接的同一句里有别人说的、判过了的
    /// （记下了放行的判断，或者在 `judging` 里：正在判）（群聊内核的 `repeats`）。不在投影里的不算。
    pub(crate) fn repeated(&self, seq: u64, now: Timestamp, window: i64, judging: &[u64]) -> bool {
        let Some(me) = self.said.get(&seq) else {
            return false;
        };
        let earlier = self
            .said
            .range(..seq)
            .rev()
            .map(|(seq, one)| spoken(*seq, one, judging));
        repeats(me.echo, &me.id, now, earlier, window)
    }

    /// 序号是 `seq` 的这一条后来有没有别人跟着说同一句（群聊内核的 `echoed`）：她回它不再引用、@。
    pub(crate) fn echoed(&self, seq: u64) -> bool {
        let Some(aimed) = self.said.get(&seq) else {
            return false;
        };
        let later = self
            .said
            .range(seq + 1..)
            .map(|(seq, one)| spoken(*seq, one, &[]));
        echoed(aimed.echo, &aimed.id, later)
    }

    /// 她已经决定要回、还没回完的几条，照先后，`except` 里的（这一次在判的）不算：谁说的（名字，没有的写号）、正文的开头（没
    /// 留下的是空的）。
    pub(crate) fn answering(&self, except: &[u64]) -> Vec<Answering> {
        self.committed
            .iter()
            .flat_map(|committed| {
                let pending = &committed.pending;
                pending.absorbed.iter().chain([&pending.msg])
            })
            .map(|msg| msg.get())
            .filter(|seq| !except.contains(seq))
            .filter_map(|seq| {
                let speaker = self.said.get(&seq)?;
                Some(Answering {
                    name: speaker
                        .name
                        .clone()
                        .unwrap_or_else(|| speaker.id.to_string()),
                    text: self.heads.get(&seq).cloned().unwrap_or_default(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
