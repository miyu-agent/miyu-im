//! 她指定下一句指向谁：@ 指定的人（施工 O-55，`onebot.md` 第一条「@ 指定的人」，2026-10-17 项目主人定，照旧版的 `qq_mention_users`）、
//! 引用指定的一条（施工 I-31，2026-10-23 项目主人定：以后的 loop 里她要自己决定指向谁）。群里她调 `mention {users}`、`quote {msg}`
//! （施工 T-4 起在驱动里认号、认是不是这个会话的），驱动交回「@ 这几个号」「引用这一条」，这里记下：她在这条线这一轮的下一句话
//! 第一段前面 @ 他们、引用那一条（`say` 用掉）。
//!
//! 记在桥的内存里，按线：换了一轮的、她这一轮没再说话的作废（下一次照新的一轮比）。

use std::collections::HashMap;

use super::Route;

/// 她指定的：按线（主线、支线的会话编号）记（回合编号，要 @ 的号，要引用的那一条）。
#[derive(Debug, Default)]
pub(super) struct Aims(HashMap<String, Aim>);

/// 一条线上她指定的。
#[derive(Debug, Default)]
pub(super) struct Aim {
    /// 第几轮指定的。
    turn: u64,
    /// 要 @ 的号。
    pub(super) users: Vec<String>,
    /// 要引用的那一条的平台编号。
    pub(super) quote: Option<String>,
}

impl Route {
    /// 线 `line` 第 `turn` 轮她指定的：用掉；记的是别的一轮的作废，交回空的。
    pub(super) fn take_aim(&mut self, line: &str, turn: u64) -> Aim {
        match self.aims.0.remove(line) {
            Some(aim) if aim.turn == turn => aim,
            _ => Aim::default(),
        }
    }

    /// 记下线 `line` 第 `turn` 轮她要 @ 的号 `users`（施工 T-4：驱动认好了交回的）：同一轮指定过要引用的留着。
    pub(super) fn mention(&mut self, line: String, turn: u64, users: &[i64]) {
        let numbers = users.iter().map(i64::to_string).collect();
        self.aim(line, turn).users = numbers;
    }

    /// 记下线 `line` 第 `turn` 轮她要引用的那一条 `msg`（施工 I-31）：同一轮指定过要 @ 的留着。
    pub(super) fn quote(&mut self, line: String, turn: u64, msg: String) {
        self.aim(line, turn).quote = Some(msg);
    }

    /// 线 `line` 第 `turn` 轮的那一份：记的是别的一轮的换成空的。
    fn aim(&mut self, line: String, turn: u64) -> &mut Aim {
        let aim = self.aims.0.entry(line).or_default();
        if aim.turn != turn {
            *aim = Aim {
                turn,
                ..Aim::default()
            };
        }
        aim
    }
}
