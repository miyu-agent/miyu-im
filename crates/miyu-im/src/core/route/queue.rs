//! 出站队列的纯逻辑（施工 O-25 中，`onebot.md` 第一条「出站队列」第 3 到 8 条，「施工时定的」第 116 条）：每个场所会话一条先进
//! 先出的队，门开着的照先后交出来，排到了期限的作废（门关着的也是）；下一次该醒的时刻；禁言到什么时候；NapCat 的回应算成功
//! 还是失败、为什么。一段可以带停顿（施工 I-26，模拟真人一条分多条发送）：离前一段过了这么久才交，没到的连同排在它后面的都
//! 等着，先后不乱。一个会话一次只交一段（施工 I-48）：交出去的那一段等到 NapCat 回了（成了、没成都算）才交下一段。NapCat
//! 并着办动作，两段挨着交的，QQ 上可能后交的先到（10-27 项目主人在群里撞到：一句分两段，中间插进了另一句）。不碰 I/O、时钟：此刻、门开没开、禁言到什么时候都由调的一方交进来（`sending.rs`），停住的钟测得了。
//!
//! 施工 I-58（10-27 项目主人：并着回的几句还是秒发）：照场所排（一个群、一个私聊一条队），不照会话：同一个群里主线、支线
//! 的话排进同一条，一段接一段。一句的第一段从这个场所上一次交出去的时刻算停顿（打字的工夫），上一次早就交了的不用等。
//!
//! 门开没开、禁言到什么时候不记在这里：禁言照投影（日志算得出，桥重启照样重建），号连没连着照连着的号（「施工时定的」第 117 条）。

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::Duration;

use miyu_client::time::Timestamp;

use super::sending::What;
use crate::chat::{CallError, DETAIL};

/// `failed` 的 `why`：NapCat 回了失败。
pub(super) const REJECTED: &str = "rejected";

/// `failed` 的 `why`：等了调用的时限还没回。
pub(super) const TIMEOUT: &str = "timeout";

/// `failed` 的 `why`：写不进、等的时候连接断了。
pub(super) const DISCONNECTED: &str = "disconnected";

/// `failed` 的 `why`：排着过了期限（第 5 条）。
pub(super) const EXPIRED: &str = "expired";

/// 排着的一段：入队的时刻、最早什么时候交、要发的。
struct Waiting<T> {
    /// 入队的时刻（本机的钟，「施工时定的」第 118 条）：期限照它算。
    at: Timestamp,
    /// 最早什么时候交，毫秒：前一段的这个时刻（排空了的照入队的时刻）加上这一段的停顿。
    ready: i64,
    /// 要发的。
    item: T,
}

/// 出站队列：场所编号 → 排着的，照入队的先后。照场所编号排（`BTreeMap`）：看一遍的先后定得住。
pub(super) struct Queue<T> {
    /// 排着的过了多久作废，毫秒（`bridge.json` 的 `queue_expire_seconds`）。
    expire: i64,
    /// 场所编号 → 排着的；排空了的拿掉。
    lines: BTreeMap<String, VecDeque<Waiting<T>>>,
    /// 交出去了、NapCat 还没回的场所（施工 I-48）：它们的下一段等着。
    out: BTreeSet<String>,
    /// 场所编号 → 上一次交出去的时刻，毫秒（施工 I-58）：排空了以后新的一段从它算停顿。
    last: BTreeMap<String, i64>,
}

/// 看一遍一个会话交出来的：过了期的、门开着交得出去的，都照入队的先后。
#[derive(Debug)]
pub(super) struct Taken<T> {
    /// 过了期的：记 `expired`，不发。
    pub(super) expired: Vec<T>,
    /// 交出去的：门开着，没过期。
    pub(super) ready: Vec<T>,
}

impl<T> Queue<T> {
    /// 空的队：排着的过了 `expire` 作废（毫秒以下的不算；大得放不下的当一直不过期，照说不会）。
    pub(super) fn new(expire: Duration) -> Queue<T> {
        Queue {
            expire: i64::try_from(expire.as_millis()).unwrap_or(i64::MAX),
            lines: BTreeMap::new(),
            out: BTreeSet::new(),
            last: BTreeMap::new(),
        }
    }

    /// 此刻是 `now`，说的时刻晚于它（毫秒）的还在期限里（施工 O-32，「出站队列」第 6 条：订阅补来的她的话照它补发）：和排着的
    /// 一个算法，此刻减期限。
    pub(super) fn since(&self, now: Timestamp) -> i64 {
        now.unix_millis().saturating_sub(self.expire)
    }

    /// 场所 `line` 在 `at` 入队了一段 `item`，离前一段停 `pause`：排在这个场所的最后。前一段还排着的从它最早交的时刻
    /// 算起；排空了的从这个场所上一次交出去的时刻算起（施工 I-58：早就交了的不用等），没交过的不等。
    pub(super) fn push(&mut self, line: &str, (at, pause): (Timestamp, Duration), item: T) {
        let sent = self.last.get(line).copied().unwrap_or(i64::MIN);
        let line = self.lines.entry(line.to_string()).or_default();
        let after = line
            .back()
            .map_or(sent, |last| last.ready.max(at.unix_millis()));
        let pause = i64::try_from(pause.as_millis()).unwrap_or(i64::MAX);
        line.push_back(Waiting {
            at,
            ready: after.saturating_add(pause),
            item,
        });
    }

    /// 有东西排着的场所，照编号排。
    pub(super) fn lines(&self) -> Vec<String> {
        self.lines.keys().cloned().collect()
    }

    /// 场所 `line` 排在最前的那一段。
    pub(super) fn front(&self, line: &str) -> Option<&T> {
        self.lines.get(line)?.front().map(|waiting| &waiting.item)
    }

    /// 看一遍场所 `session`，此刻是 `now`，门开着是 `open`：入队时刻加期限不晚于此刻的交出来作废（门关着的也是）；门开着、
    /// 上一段已经回了的，交出排在最前的那一段（到了最早交的时刻的），记下这个场所有一段在外面（施工 I-48）、这一刻交的（施工
    /// I-58），等 [`Queue::answered`] 再交下一段。门关着、没过期的留着。
    pub(super) fn take(&mut self, session: &str, now: Timestamp, open: bool) -> Taken<T> {
        let mut taken = Taken {
            expired: Vec::new(),
            ready: Vec::new(),
        };
        let expire = self.expire;
        let Some(line) = self.lines.get_mut(session) else {
            return taken;
        };
        let mut kept = VecDeque::new();
        let mut free = open && !self.out.contains(session);
        for waiting in line.drain(..) {
            if deadline(&waiting, expire) <= now.unix_millis() {
                taken.expired.push(waiting.item);
            } else if free && kept.is_empty() && waiting.ready <= now.unix_millis() {
                taken.ready.push(waiting.item);
                self.out.insert(session.to_string());
                self.last.insert(session.to_string(), now.unix_millis());
                free = false;
            } else {
                kept.push_back(waiting);
            }
        }
        if kept.is_empty() {
            self.lines.remove(session);
        } else {
            *line = kept;
        }
        taken
    }

    /// 场所 `session` 交出去的那一段有了结局（施工 I-48）：下一段可以交了。
    pub(super) fn answered(&mut self, session: &str) {
        self.out.remove(session);
    }

    /// 下一次该看一遍的时刻（第 8 条）：排着的里最早的过期时刻；会话被禁言着的（`muted` 交回禁言到期的时刻，没禁言着的是空的）
    /// 禁言到期的时刻也算；每个会话排在最前的那一段还没到最早交的时刻（此刻是 `now`）的，那个时刻也算（到了的不算：门关着时
    /// 算它会一直醒）。没有排着的是空的：不用醒。
    pub(super) fn wake(
        &self,
        now: Timestamp,
        muted: impl Fn(&str) -> Option<Timestamp>,
    ) -> Option<Timestamp> {
        let deadlines = self
            .lines
            .values()
            .flatten()
            .map(|waiting| deadline(waiting, self.expire));
        let readies = self
            .lines
            .values()
            .filter_map(|line| line.front())
            .map(|waiting| waiting.ready)
            .filter(|ready| *ready > now.unix_millis());
        let ends = self
            .lines
            .keys()
            .filter_map(|session| muted(session))
            .map(Timestamp::unix_millis);
        let first = deadlines.chain(ends).chain(readies).min()?;
        Timestamp::from_unix_millis(first)
    }
}

/// 排着的这一段到期的时刻，毫秒：入队的时刻加 `expire` 毫秒。
fn deadline<T>(waiting: &Waiting<T>, expire: i64) -> i64 {
    waiting.at.unix_millis().saturating_add(expire)
}

/// 禁言到什么时候（第 7 条，「施工时定的」第 122 条）：此刻 `now` 加 `seconds` 秒。算出来超出范围的是空的。
pub(super) fn until(now: Timestamp, seconds: u64) -> Option<Timestamp> {
    let millis = i64::try_from(seconds).ok()?.checked_mul(1000)?;
    Timestamp::from_unix_millis(now.unix_millis().checked_add(millis)?)
}

/// 交给 NapCat 的一段的结局（第 4 条）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Ending {
    /// 成了：NapCat 回的 `message_id`，没回的是空的。
    Sent(Option<String>),
    /// 没成：`why`（[`REJECTED`]、[`TIMEOUT`]、[`DISCONNECTED`]），NapCat 说的原因（只有回了失败的有）。
    Failed(&'static str, Option<String>),
}

/// 一次调用的结果算成什么（第 4 条，「施工时定的」第 120 条）：回了 `ok` 的成了，带回的 `message_id`；回了失败的是
/// [`REJECTED`]，原因照回应的 `message` 去掉首尾空白、截到 [`DETAIL`] 个字符，空的、没有的不带；等不到是 [`TIMEOUT`]；
/// 连接断了是 [`DISCONNECTED`]。
pub(super) fn ending(result: &Result<Option<String>, CallError>) -> Ending {
    match result {
        Ok(msg) => Ending::Sent(msg.clone()),
        Err(CallError::Failed { message, .. }) => {
            let detail = (!message.is_empty()).then(|| message.chars().take(DETAIL).collect());
            Ending::Failed(REJECTED, detail)
        }
        Err(CallError::Timeout) => Ending::Failed(TIMEOUT, None),
        Err(CallError::Closed) => Ending::Failed(DISCONNECTED, None),
        // 这个平台发不了这一样（施工 I-82）：照回了失败记。
        Err(CallError::Unsupported) => Ending::Failed(REJECTED, Some("unsupported".to_string())),
    }
}

#[cfg(test)]
mod tests;

/// 首页「今天」数的是她说的（施工 I-6 下）：一段话、一张图、长文的第一页；回执、长文后面几页不算。
pub(super) fn said(what: &What) -> bool {
    match what {
        What::Reply { .. } | What::Image(_) => true,
        What::Page(page) => page.first,
        What::Receipt { .. } => false,
    }
}
