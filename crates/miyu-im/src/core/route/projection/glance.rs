//! 时不时看一眼要的几样（施工 I-89，`crate::glance`）：这个群这时有没有一轮在跑、人说的话当过一轮的触发没有。

use miyu_client::events::Event;
use miyu_client::ids::Seq;

use super::Projection;

impl Projection {
    /// 主线或者哪条支线正在跑一轮：到点的看一眼顺延到它结束（设计 05 第一节第 3 条）。
    pub(in crate::core::route) fn busy(&self) -> bool {
        self.main.running.is_some() || !self.lanes.is_empty()
    }

    /// 人说的话：主线序号、当过一轮的触发没有，照先后。
    pub(in crate::core::route) fn heard(&self) -> impl Iterator<Item = (u64, bool)> + '_ {
        self.said
            .iter()
            .map(|(seq, speaker)| (*seq, speaker.answered))
    }

    /// 这几条当了一轮的触发（开轮、并进一轮）：记下，看一眼不再交。
    pub(super) fn answered(&mut self, triggers: &[Seq]) {
        for seq in triggers {
            if let Some(speaker) = self.said.get_mut(&seq.get()) {
                speaker.answered = true;
            }
        }
    }
}

/// 这一轮是不是看一眼开的（施工 I-187）：`turn.started` 的 `cause` 是看一眼的命令编号。
pub(super) fn opened(event: &Event) -> bool {
    event
        .cause
        .as_ref()
        .is_some_and(|cause| cause.as_str().starts_with(super::super::glance::OPENED))
}
