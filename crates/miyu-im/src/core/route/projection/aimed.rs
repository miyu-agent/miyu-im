//! 交给 NapCat 的那一刻再定带不带引用、@（施工 I-64，`onebot.md`「群里怎么叫她」第 9 条 `since`）：她的位置照交出去那一刻算，
//! 打字停顿（I-49、I-58）、排队那几秒里进来的人话，群里排在她前面。

use miyu_client::ids::ExternalId;

use super::Projection;

/// 她发出去的话留最近几条的序号（施工 I-162）。
pub(super) const OWN_KEPT: usize = 200;

impl Projection {
    /// 序号是 `seq` 的那一条以后，群里隔了几条：不是 `sender` 说的人话，加上她已经发出去的话（施工 I-162，项目主人 11-05：她在
    /// 中间回了别人两句，再回这一条不引用就分不清在回谁）。带引用的只有一句话的第一段，交出去那一刻这一句自己还一段都没发，
    /// 所以算进去的都是她回别的话的。
    pub(super) fn others_after(&self, seq: u64, sender: &ExternalId) -> u64 {
        let later = self.said.range(seq + 1..);
        let people = later.filter(|(_, speaker)| speaker.id != *sender).count();
        (people + self.own_sent.range(seq + 1..).count()) as u64
    }

    /// 她发出去了一段，记的是序号 `seq`（`venue.delivered`）：留最近 [`OWN_KEPT`] 条。
    pub(super) fn sent_own(&mut self, seq: u64) {
        self.own_sent.insert(seq);
        while self.own_sent.len() > OWN_KEPT {
            self.own_sent.pop_first();
        }
    }

    /// 群里最后一条是不是她的：她说的最后一段排在最后一条人说的话后面。她的位置照说它的 `message.assistant`（回执晚到，
    /// 发出去到回执之间进来的人话在群里排在她后面，O-25 上），另照交给 NapCat 的那一刻（[`Self::handed`]）。
    pub(super) fn last_is_own(&self) -> bool {
        let last_said = self.said.last_key_value().map_or(0, |(seq, _)| *seq);
        self.own > last_said || self.handed.is_some_and(|after| after >= last_said)
    }

    /// 她回序号是 `seq`、发的人是 `sender` 的那一条，这一刻群里的动静：那之后别人说了几条、群里最后一条是不是她的。
    pub(in crate::core::route) fn since_aim(&self, seq: u64, sender: &ExternalId) -> (u64, bool) {
        (self.others_after(seq, sender), self.last_is_own())
    }

    /// 她刚把一段交给 NapCat：这一刻收到的人话都排在她前面。
    pub(in crate::core::route) fn handed(&mut self) {
        let last_said = self.said.last_key_value().map_or(0, |(seq, _)| *seq);
        self.handed = Some(
            self.handed
                .map_or(last_said, |before| before.max(last_said)),
        );
    }
}
