//! @ 指定的人（施工 O-55，`onebot.md` 第一条「@ 指定的人」，2026-10-17 项目主人定，照旧版的 `qq_mention_users`）：群里她调
//! `mention {users}`（施工 T-4 起在驱动里认号、认是不是这个群的成员），驱动交回「@ 这几个号」，这里记下：她在这条线这一轮的下一句
//! 话第一段前面 @ 他们（`say` 用掉）。
//!
//! 记在桥的内存里，按线：换了一轮的、她这一轮没再说话的作废（下一次照新的一轮比）。

use std::collections::HashMap;

use super::Route;

/// 她要 @ 的：按线（主线、支线的会话编号）记（回合编号，号）。
#[derive(Debug, Default)]
pub(super) struct Mentions(HashMap<String, (u64, Vec<String>)>);

impl Route {
    /// 线 `line` 第 `turn` 轮她要 @ 的号：用掉；记的是别的一轮的作废，交回空的。
    pub(super) fn take_mentions(&mut self, line: &str, turn: u64) -> Vec<String> {
        match self.mentions.0.remove(line) {
            Some((kept, users)) if kept == turn => users,
            _ => Vec::new(),
        }
    }

    /// 记下线 `line` 第 `turn` 轮她要 @ 的号 `users`（施工 T-4：驱动认好了交回的）。
    pub(super) fn mention(&mut self, line: String, turn: u64, users: &[i64]) {
        let numbers = users.iter().map(i64::to_string).collect();
        self.mentions.0.insert(line, (turn, numbers));
    }
}
