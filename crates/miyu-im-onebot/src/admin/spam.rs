//! 刷屏处理（施工 Q-5，设计 02 第一节第 7 条）：同一个人发同样的话，**他自己连着发**（别人插话不打断）[`COUNT`] 次及以上、头一条到这一条
//! 在这个群设的时间窗以内，就算刷屏：撤回重复的那几条（头一条留着），禁言（这个群设的时长），记进管理记录（谁做的是刷屏处理），给
//! 会话记一句 `moderated`。不让 AI 另开一轮：第 [`COUNT`] 条起不交给 im（2026-10-20 项目主人定），它只看到那一句。
//!
//! 只在群管理在这个群生效的时候管（插件开着、这个群开了刷屏处理、她是管理员或群主）；发的人是群主、管理员的不管（禁不了）。字照
//! 去掉首尾空白比；没有字的（只有图、表情）不算，也打断连着。她是不是管理员照问过的记一会儿（[`RANKED`]）：开了刷屏处理的群里
//! 一有人说话，不知道的、过期了的先另起任务问；到了第 [`COUNT`] 条还不知道的，那一条照交给 im，问过了再办。

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use miyu_im::chat::{Link, Msg, Posted, Rank, To};
use miyu_im::driver::Gate;
use miyu_im::toolset::{NAME, clean};

use super::groups::Cached;
use super::records::{self, Action, By, Record};
use super::sync::spoken;
use crate::admin::{self, ID};

/// 连着发几次算刷屏。
pub(crate) const COUNT: usize = 3;

/// 问过她是不是管理员，记多久。
const RANKED: Duration = Duration::from_secs(600);

/// 一个人正连着发的同一句。
#[derive(Debug)]
struct Streak {
    /// 那一句（去掉首尾空白）。
    text: String,
    /// 头一条的时刻。
    first: Instant,
    /// 这几条的编号，照先后。
    messages: Vec<i64>,
}

/// 刷屏了要做的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Flood {
    /// 要撤回的那几条（头一条以外的）。
    pub recall: Vec<i64>,
    /// 这一条刚到 [`COUNT`]：要禁言、记录、告诉她；再往后的只撤回。
    pub fresh: bool,
}

/// 每个群每个人正连着发的（纯逻辑）。
#[derive(Debug, Default)]
pub(crate) struct Streaks(HashMap<(i64, i64), Streak>);

impl Streaks {
    /// 群 `group` 里 `user` 发了编号 `message` 的 `text`（此刻 `now`，时间窗 `window`）：到了 [`COUNT`] 的交回要做的。
    pub(crate) fn seen(
        &mut self,
        (group, user): (i64, i64),
        (text, message): (&str, i64),
        now: Instant,
        window: Duration,
    ) -> Option<Flood> {
        let text = text.trim();
        if text.is_empty() {
            self.0.remove(&(group, user));
            return None;
        }
        let streak = self.0.entry((group, user)).or_insert_with(|| Streak {
            text: String::new(),
            first: now,
            messages: Vec::new(),
        });
        if streak.text != text || now.duration_since(streak.first) > window {
            *streak = Streak {
                text: text.to_string(),
                first: now,
                messages: Vec::new(),
            };
        }
        streak.messages.push(message);
        match streak.messages.len() {
            count if count < COUNT => None,
            count if count == COUNT => Some(Flood {
                recall: streak.messages[1..].to_vec(),
                fresh: true,
            }),
            _ => Some(Flood {
                recall: vec![message],
                fresh: false,
            }),
        }
    }
}

/// 一条连接上的刷屏处理：连着发的、按群的设置、她是不是管理员。
pub(crate) struct Spamming {
    gate: Arc<Gate>,
    link: Arc<dyn Link>,
    streaks: Streaks,
    groups: Cached,
    /// 群号 → （她是不是管理员，问到的时刻）。另起的任务问过了写回来。
    ranked: Arc<Mutex<HashMap<i64, (bool, Instant)>>>,
    /// 正在问的群。
    asking: Arc<Mutex<HashSet<i64>>>,
}

impl Spamming {
    /// 经这条连接 `link` 办的一份。
    pub(crate) fn new(gate: &Arc<Gate>, link: &Arc<dyn Link>) -> Spamming {
        Spamming {
            gate: Arc::clone(gate),
            link: Arc::clone(link),
            streaks: Streaks::default(),
            groups: Cached::default(),
            ranked: Arc::default(),
            asking: Arc::default(),
        }
    }

    /// 群 `group` 里的一条 `posted`：刷屏了的另起任务办（模块的说明）；交回这一条还交不交给 im（她是管理员、这一条到了 [`COUNT`]
    /// 及以后的不交）。
    pub(crate) fn seen(&mut self, group: i64, posted: &Posted) -> bool {
        if posted.user == posted.bot
            || matches!(posted.rank, Some(Rank::Owner | Rank::Admin))
            || !self.gate.active(ID, To::Group(group))
        {
            return true;
        }
        let spam = self.groups.group(self.gate.store(), group).spam;
        if !spam.on {
            return true;
        }
        self.ask_rank(posted.bot, group);
        let Some(flood) = self.streaks.seen(
            (group, posted.user),
            (&posted.text, posted.message_id),
            Instant::now(),
            Duration::from_secs(spam.window),
        ) else {
            return true;
        };
        let known = self
            .ranked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&group)
            .filter(|(_, at)| at.elapsed() < RANKED)
            .map(|(admin, _)| *admin);
        if known == Some(false) {
            return true;
        }
        let job = Job {
            gate: Arc::clone(&self.gate),
            link: Arc::clone(&self.link),
            ranked: Arc::clone(&self.ranked),
            store: self.gate.store().to_path_buf(),
        };
        let (who, user) = (posted.name.clone(), posted.user);
        let bot = posted.bot;
        tokio::spawn(async move {
            job.handled((bot, group), (user, who), flood, spam.mute)
                .await
        });
        known != Some(true)
    }
}

impl Spamming {
    /// 她在群 `group` 里是不是管理员还不知道、记的过期了的，另起任务问一次（同一个群问着的不再问）：到了第 [`COUNT`] 条一般已经
    /// 知道，那一条照它定交不交给 im。
    fn ask_rank(&self, bot: i64, group: i64) {
        {
            let mut ranked = self.ranked.lock().unwrap_or_else(PoisonError::into_inner);
            let fresh = ranked
                .get(&group)
                .is_some_and(|(_, at)| at.elapsed() < RANKED);
            if fresh
                || !self
                    .asking
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .insert(group)
            {
                return;
            }
            ranked.remove(&group);
        }
        let (link, ranked, asking) = (
            Arc::clone(&self.link),
            Arc::clone(&self.ranked),
            Arc::clone(&self.asking),
        );
        tokio::spawn(async move {
            if let Ok(admin) = admin::admin_here(link.as_ref(), group, bot).await {
                ranked
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .insert(group, (admin, Instant::now()));
            }
            asking
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&group);
        });
    }
}

/// 另起任务办一次刷屏要的几样。
struct Job {
    gate: Arc<Gate>,
    link: Arc<dyn Link>,
    ranked: Arc<Mutex<HashMap<i64, (bool, Instant)>>>,
    store: PathBuf,
}

impl Job {
    /// 她在群里是管理员才办：撤回重复的；刚到 [`COUNT`] 的禁言 `mute` 秒，成了的记进管理记录、告诉她。
    async fn handled(
        &self,
        (bot, group): (i64, i64),
        (user, name): (i64, Option<String>),
        flood: Flood,
        mute: u64,
    ) {
        let admin = matches!(
            admin::admin_here(self.link.as_ref(), group, bot).await,
            Ok(true)
        );
        self.ranked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(group, (admin, Instant::now()));
        if !admin {
            return;
        }
        for message in &flood.recall {
            if let Err(error) = self.link.recall(Msg::Number(*message)).await {
                tracing::warn!(target: miyu_im::TARGET, group, error = ?error, "flood message not recalled");
            }
        }
        if !flood.fresh {
            return;
        }
        // 撤回成了、禁言没成的记一行（施工 I-17：以前悄悄放过）。
        if let Err(error) = self.link.mute(group, user, mute).await {
            tracing::warn!(target: miyu_im::TARGET, group, user, error = ?error, "flood sender not muted");
            return;
        }
        let config = self.gate.config().borrow().clone();
        let name = name.and_then(|name| clean(&name, NAME));
        if admin::recording(&config) {
            let record = Record {
                at: records::now(),
                action: Action::Mute,
                user,
                name: name.clone(),
                by: By::Spam,
                seconds: Some(mute),
                reason: None,
                block: false,
                verdict: None,
                comment: None,
            };
            records::append(self.store.clone(), group, record, admin::keep(&config)).await;
        }
        let who = name.unwrap_or_else(|| user.to_string());
        let duration = spoken(mute);
        let fields = [("who", who.as_str()), ("duration", &duration)];
        self.gate
            .note((bot, To::Group(group)), "moderated", "spam-muted", &fields)
            .await;
    }
}

#[cfg(test)]
mod tests;
