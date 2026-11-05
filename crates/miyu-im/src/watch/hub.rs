//! 实时日志的汇总（设计 10 第二节，施工 I-160 中）：桥收到的场所会话事件（主线、支线）照场所整理成行，留最近的一段，推给看着的。
//!
//! - 每个场所留最近 [`KEEP`] 行定下来的（写着的增量不留）；桥重启从空的开始。
//! - 记着最近 [`NAMES`] 条消息是谁说的（开轮、判断照触发的序号找名字），工具名对显示名（照结果里的 `display_name` 记）。
//! - 看的人照场所数着（[`Hub::watch`] 交回的 [`Watching`] 放下就少一个）：没人看的场所，增量（`model.delta`）不整理、不推，
//!   定下来的行照留不推。
//! - 推的是结构化的行（[`Batch`]），说成话在看的那一头。
//! - 场所的名字照收到的消息的 `venue.place`（群名）、`venue.name`（私聊的人）记，[`Hub::venues`] 交出来给命令行认 `qq:号`。
//! - 场所照短编号记（[`short`]：去掉中间的机器人号，`qq:3927564101:group:130515298` 记成 `qq:group:130515298`）：看的人
//!   不知道是哪个号收的。

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;

use super::{Item, Line, lines};

/// 每个场所留几行。
pub const KEEP: usize = 300;

/// 每个场所记最近几条消息的名字。
pub const NAMES: usize = 200;

/// 推的队列攒几批：看的一头跟不上的丢最旧的（它照 `lagged` 再要一遍最近的）。
const QUEUE: usize = 256;

/// 记几个调用编号对工具名。
const CALLS: usize = 64;

/// 推给看着的一批：哪个场所、几行。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Batch {
    /// 场所编号。
    pub venue: String,
    /// 场所的名字（群名、私聊的人），还不知道的是空的。
    pub name: String,
    /// 几行，照先后。
    pub lines: Vec<Line>,
}

/// 一个场所：名字、留着的行、最近几条消息是谁说的。
#[derive(Default)]
struct Place {
    name: String,
    lines: VecDeque<Line>,
    names: VecDeque<(u64, String)>,
}

/// 锁着的那一份。
#[derive(Default)]
struct Inner {
    places: HashMap<String, Place>,
    /// 场所编号 → 看的人数；看全部的记在空串下面。
    watching: HashMap<String, usize>,
    /// 工具名 → 显示名。
    tools: HashMap<String, String>,
    /// 调用编号 → 工具名（照先后，留最近 [`CALLS`] 个）。
    calls: VecDeque<(String, String)>,
}

/// 汇总：桥的路由往里交，页面通道、命令行从里面看。
pub struct Hub {
    inner: Mutex<Inner>,
    pushed: broadcast::Sender<Batch>,
}

impl Default for Hub {
    fn default() -> Hub {
        Hub {
            inner: Mutex::default(),
            pushed: broadcast::channel(QUEUE).0,
        }
    }
}

/// 一个看着的：放下就不再算它。
pub struct Watching {
    hub: Arc<Hub>,
    key: String,
}

impl Drop for Watching {
    fn drop(&mut self) {
        let mut inner = self.hub.lock();
        if let Some(count) = inner.watching.get_mut(&self.key) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                inner.watching.remove(&self.key);
            }
        }
    }
}

impl Hub {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// 场所 `venue` 的线 `lane`（会话编号）推来一条事件 `event`：整理成行，定下来的留着，有人看的推出去。
    pub fn heard(&self, venue: &str, lane: &str, event: &Value) {
        let short = short(venue);
        let venue = short.as_str();
        let delta = event["kind"] == "model.delta";
        let mut inner = self.lock();
        let watched = inner.watching.contains_key(venue) || inner.watching.contains_key("");
        if delta && !watched {
            return;
        }
        remember(&mut inner, event);
        let place = inner.places.entry(venue.to_string()).or_default();
        if event["kind"] == "message.user" {
            let venue_of = &event["body"]["venue"];
            if let Some(name) = venue_of["place"]
                .as_str()
                .or(venue_of["name"].as_str())
                .filter(|name| !name.is_empty())
            {
                name.clone_into(&mut place.name);
            }
        }
        let names = &place.names;
        let found = |seq: u64| {
            names
                .iter()
                .rev()
                .find(|(at, _)| *at == seq)
                .map(|(_, name)| name.clone())
        };
        let made = lines(event, lane, &found);
        if let (
            Some(seq),
            Some(Line {
                item: Item::Heard { who, .. },
                ..
            }),
        ) = (event["seq"].as_u64(), made.first())
        {
            place.names.push_back((seq, who.clone()));
            if place.names.len() > NAMES {
                place.names.pop_front();
            }
        }
        if made.is_empty() {
            return;
        }
        for line in made.iter().filter(|line| settled(&line.item)) {
            place.lines.push_back(line.clone());
            if place.lines.len() > KEEP {
                place.lines.pop_front();
            }
        }
        if watched {
            let batch = Batch {
                venue: venue.to_string(),
                name: place.name.clone(),
                lines: made,
            };
            // 没有收的（看的都放下了）照说不会：数着的和收着的是一起放下的。真没人收也不算错，不用管。
            if self.pushed.send(batch).is_err() {}
        }
    }

    /// 开始看场所 `venue`（空的是全部）：交回最近留着的（几个场所的并在一起照时刻排，最多 [`KEEP`] 行）、收推送的一头和
    /// 看着的凭据。
    pub fn watch(
        self: &Arc<Hub>,
        venue: Option<&str>,
    ) -> (Vec<Batch>, broadcast::Receiver<Batch>, Watching) {
        let key = venue.unwrap_or_default().to_string();
        let mut inner = self.lock();
        *inner.watching.entry(key.clone()).or_default() += 1;
        let receiver = self.pushed.subscribe();
        let recent = recent(&inner, venue);
        drop(inner);
        (
            recent,
            receiver,
            Watching {
                hub: Arc::clone(self),
                key,
            },
        )
    }

    /// 最近留着的（看的一头跟不上、丢了推送时再要一遍）。
    pub fn recent(&self, venue: Option<&str>) -> Vec<Batch> {
        recent(&self.lock(), venue)
    }

    /// 见过的场所：编号、名字，照编号排。
    pub fn venues(&self) -> Vec<(String, String)> {
        let inner = self.lock();
        let mut all: Vec<_> = inner
            .places
            .iter()
            .map(|(venue, place)| (venue.clone(), place.name.clone()))
            .collect();
        all.sort();
        all
    }

    /// 工具名对的显示名（结果里见过的）。
    pub fn tool(&self, name: &str) -> Option<String> {
        self.lock().tools.get(name).cloned()
    }

    /// 有没有人在看场所 `venue`（看全部的也算）。
    pub fn watched(&self, venue: &str) -> bool {
        let inner = self.lock();
        inner.watching.contains_key(venue) || inner.watching.contains_key("")
    }
}

/// 场所的短编号：`<平台>:<号>:<group|private>:<号>` 去掉机器人号，别的写法照原样。
#[must_use]
pub fn short(venue: &str) -> String {
    match venue.split(':').collect::<Vec<_>>().as_slice() {
        [platform, _bot, kind @ ("group" | "private"), id] => format!("{platform}:{kind}:{id}"),
        _ => venue.to_string(),
    }
}

/// 定下来的行：写着的块、接上的字不留。
fn settled(item: &Item) -> bool {
    match item {
        Item::Wrote { done, .. } => *done,
        Item::Appended { .. } => false,
        _ => true,
    }
}

/// 记下调用编号对工具名、工具名对显示名。
fn remember(inner: &mut Inner, event: &Value) {
    let body = &event["body"];
    match event["kind"].as_str() {
        Some("message.assistant") => {
            for call in body["blocks"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
            {
                if let (Some(id), Some(name)) = (call["call_id"].as_str(), call["name"].as_str()) {
                    inner.calls.push_back((id.to_string(), name.to_string()));
                    if inner.calls.len() > CALLS {
                        inner.calls.pop_front();
                    }
                }
            }
        }
        Some("tool.result") => {
            let shown = ["zh", "en"]
                .iter()
                .find_map(|lang| body["display_name"][lang].as_str());
            let name = body["call_id"]
                .as_str()
                .and_then(|id| inner.calls.iter().rev().find(|(call, _)| call == id))
                .map(|(_, name)| name.clone());
            if let (Some(name), Some(shown)) = (name, shown) {
                inner.tools.insert(name, shown.to_string());
            }
        }
        _ => {}
    }
}

/// 场所 `venue`（空的是全部）留着的，几个场所的照时刻并成最多 [`KEEP`] 行，一个场所一批。
fn recent(inner: &Inner, venue: Option<&str>) -> Vec<Batch> {
    let mut all: Vec<(&String, &Place, &Line)> = inner
        .places
        .iter()
        .filter(|(key, _)| venue.is_none_or(|venue| venue == key.as_str()))
        .flat_map(|(key, place)| place.lines.iter().map(move |line| (key, place, line)))
        .collect();
    all.sort_by_key(|(_, _, line)| line.at);
    let skip = all.len().saturating_sub(KEEP);
    let mut batches: Vec<Batch> = Vec::new();
    for (key, place, line) in all.into_iter().skip(skip) {
        match batches.last_mut() {
            Some(batch) if &batch.venue == key => batch.lines.push(line.clone()),
            _ => batches.push(Batch {
                venue: key.clone(),
                name: place.name.clone(),
                lines: vec![line.clone()],
            }),
        }
    }
    batches
}

#[cfg(test)]
mod tests;
