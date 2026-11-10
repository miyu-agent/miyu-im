//! 会话的定时任务（设计 12 第一节第 3 条，施工 I-198）的纯逻辑：定时里存什么（[`Task`]）。计时用核心的定时（`schedule.*`，
//! `target` 是 `{"package": {}}`，同时不时看一眼），到点核心推 `schedule.fired`，im 在这张卡的会话里开一轮；接线在
//! `crate::core::route` 的 `tasks`。
//!
//! 定时的 `prompt` 是一份 JSON，`kind` 是 `task`（看一眼的那一份没有这一格，照它分）：哪张卡（平台、只管一个号的写号、群还是
//! 私聊、群号或者对方的号）、要做什么。到点原样交回，im 不另外记状态。照卡记、不照场所编号：卡管整个平台的，机器人号换了任务还在。

use serde::{Deserialize, Serialize};

/// `prompt` 里的 `kind`。
const KIND: &str = "task";

/// 写进定时的那一份。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    /// 认得出是任务的那一格：[`KIND`]。
    kind: String,
    /// 平台：驱动报的名字。
    pub platform: String,
    /// 卡只管一个机器人号的写它；管整个平台的不写。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot: Option<String>,
    /// 是群（不是的是私聊）。
    pub group: bool,
    /// 群号，或者私聊对方的号。
    pub id: String,
    /// 要做什么：人写的那段。
    pub text: String,
}

impl Task {
    /// 卡（平台 `platform`、号 `bot`、群不群 `group`、`id`）上要做 `text` 的任务。
    #[must_use]
    pub fn new(
        (platform, bot): (&str, Option<&str>),
        (group, id): (bool, &str),
        text: &str,
    ) -> Task {
        Task {
            kind: KIND.to_string(),
            platform: platform.to_string(),
            bot: bot.map(str::to_string),
            group,
            id: id.to_string(),
            text: text.to_string(),
        }
    }

    /// 写成定时的 `prompt`。
    #[must_use]
    pub fn text(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// 读定时交回来的 `prompt`；不是任务的（看一眼的、别的写法、手造的）是空的。
    #[must_use]
    pub fn read(text: &str) -> Option<Task> {
        serde_json::from_str::<Task>(text)
            .ok()
            .filter(|task| task.kind == KIND)
    }

    /// 同一张卡上要做 `text` 的任务。
    #[must_use]
    pub fn with_text(&self, text: &str) -> Task {
        Task {
            text: text.to_string(),
            ..self.clone()
        }
    }

    /// 是不是同一张卡的（要做什么不比）。
    #[must_use]
    pub fn same_card(&self, other: &Task) -> bool {
        (&self.platform, &self.bot, self.group, &self.id)
            == (&other.platform, &other.bot, other.group, &other.id)
    }
}

#[cfg(test)]
mod tests {
    use super::Task;
    use crate::glance::Prompt;

    #[test]
    fn a_task_reads_back_and_a_glance_is_not_a_task() {
        let task = Task::new(("qq", Some("30003")), (true, "778"), "每天报一下天气");
        assert_eq!(Task::read(&task.text()), Some(task.clone()));
        let whole = Task::new(("qq", None), (true, "778"), "别的事");
        assert!(!whole.text().contains("bot"), "管整个平台的不写号");
        assert!(
            !task.same_card(&whole),
            "只管一个号的和管整个平台的是两张卡"
        );
        assert!(whole.same_card(&Task::new(("qq", None), (true, "778"), "x")));
        let glance = Prompt {
            venue: "qq:30003:group:778".to_string(),
            after: 1,
            seconds: 60,
            hers: false,
        };
        assert_eq!(Task::read(&glance.text()), None, "看一眼的不是任务");
        assert_eq!(Prompt::read(&task.text()), None, "任务也不当看一眼读");
        assert_eq!(Task::read("每天八点"), None);
    }
}
