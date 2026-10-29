//! 补拉消息（施工 I-15，设计 18 第十一节「补拉消息」，驱动提供）：桥断线、重启时漏掉的群消息，号连上以后从平台补回来，交给 im
//! 记进聊天记录（`Event::Backfilled`，只进聊天记录，不判、不开回合）。
//!
//! 1. 每个号记着每个群收到的最后一条的平台时刻（[`Seen`]，`<状态>/backfill/<号>.json`）：实时收到的群消息都记，隔
//!    [`FLUSH`] 写一次盘，断开时再写一次。没记过的群不补（桥第一次见它以前的不算漏掉的）。
//! 2. 号认出来以后（[`start`]）另起任务，照记着的群一个一个问 `get_group_msg_history`（最近 [`COUNT`] 条），平台时刻不早于记着的、
//!    不是她自己发的，照先后交给 im。同一秒的、补拉时实时又收到的会重：命令编号同实时的那一条，核心只记一次。
//! 3. 补满了 [`COUNT`] 条的（断得久，前面可能还有）记一行 `INFO backfill truncated`；问不到的记一行 `WARN`，这个群这次不补。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::onebot::{self, Event, Frame};
use miyu_im::TARGET;
use miyu_im::chat::{Id, Link, Posted};
use miyu_im::driver::Gate;

/// 一个群最多补几条。
pub(crate) const COUNT: usize = 30;

/// 记着的最多隔多久写一次盘：桥崩了少记的那一段下次多补几条，核心照命令编号去重。
const FLUSH: Duration = Duration::from_secs(10);

/// 一个号记着的：群号 → 收到的最后一条的平台时刻（整数秒）。
#[derive(Debug)]
pub(crate) struct Seen {
    /// 写在哪。
    path: PathBuf,
    /// 群号 → 平台时刻。
    groups: BTreeMap<Id, i64>,
    /// 有没写盘的。
    dirty: bool,
    /// 上次写盘的时刻。
    flushed: Instant,
}

impl Seen {
    /// 号 `bot` 记着的（状态目录 `store` 下）；没有的、读不出的是空的。
    pub(crate) fn load(store: &Path, bot: Id) -> Seen {
        let path = store.join("backfill").join(format!("{bot}.json"));
        let groups = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<BTreeMap<String, i64>>(&bytes).ok())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(group, time)| Some((Id::new(&group)?, time)))
            .collect();
        Seen {
            path,
            groups,
            dirty: false,
            flushed: Instant::now(),
        }
    }

    /// 记着的群和时刻，照群号。
    pub(crate) fn groups(&self) -> Vec<(Id, i64)> {
        self.groups
            .iter()
            .map(|(group, &time)| (group.clone(), time))
            .collect()
    }

    /// 群 `group` 收到一条平台时刻是 `time` 的：只往后挪；隔够了 [`FLUSH`] 写盘。没带时刻的（0）不记。
    pub(crate) fn note(&mut self, group: Id, time: i64) {
        if time <= 0 {
            return;
        }
        let last = self.groups.entry(group).or_default();
        if time > *last {
            *last = time;
            self.dirty = true;
        }
        if self.dirty && self.flushed.elapsed() >= FLUSH {
            self.flush();
        }
    }

    /// 有没写盘的就写；写不进的记一行，下次再写。
    pub(crate) fn flush(&mut self) {
        if !self.dirty {
            return;
        }
        self.flushed = Instant::now();
        match write(&self.path, &self.groups) {
            Ok(()) => self.dirty = false,
            Err(error) => {
                tracing::warn!(target: TARGET, error = %error, "backfill marks not saved")
            }
        }
    }
}

/// 写进 `path`：先写旁边的 `.partial` 再换名。
fn write(path: &Path, groups: &BTreeMap<Id, i64>) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let named: BTreeMap<String, i64> = groups
        .iter()
        .map(|(group, time)| (group.to_string(), *time))
        .collect();
    let partial = path.with_extension("json.partial");
    std::fs::write(
        &partial,
        serde_json::to_vec(&named).map_err(std::io::Error::other)?,
    )?;
    std::fs::rename(&partial, path)
}

/// 号 `bot` 连上了：另起任务照 `groups`（[`Seen::groups`]）补。
pub(crate) fn start(gate: &Arc<Gate>, link: &Arc<dyn Link>, bot: Id, groups: Vec<(Id, i64)>) {
    if groups.is_empty() {
        return;
    }
    let (gate, link) = (Arc::clone(gate), Arc::clone(link));
    tokio::spawn(async move {
        for (group, since) in groups {
            let params = json!({"group_id": group, "count": COUNT});
            let reply = match link.raw("get_group_msg_history", params).await {
                Ok(reply) if reply["status"] == "ok" => reply,
                Ok(reply) => {
                    let said = reply["message"].as_str().unwrap_or_default().to_string();
                    tracing::warn!(target: TARGET, group = %group, said, "backfill not fetched");
                    continue;
                }
                Err(error) => {
                    tracing::warn!(target: TARGET, group = %group, error = ?error, "backfill not fetched");
                    continue;
                }
            };
            let (missed, full) = missed(
                &reply["data"]["messages"],
                (bot.clone(), group.clone()),
                since,
            );
            if full {
                tracing::info!(target: TARGET, group = %group, count = COUNT, "backfill truncated");
            }
            for posted in missed {
                if !gate
                    .inbound(Event::Backfilled {
                        group: group.clone(),
                        posted,
                    })
                    .await
                {
                    return;
                }
            }
        }
    });
}

/// 平台交回的最近几条（`messages`）里漏掉的：号 `bot` 收到的、群 `group` 的、平台时刻不早于 `since` 的、不是她自己发的，照
/// 时刻、编号排好；另交回前面是不是可能还有（交回了 [`COUNT`] 条，最早的一条也晚于 `since`）。读不出的那一条不要。
pub(crate) fn missed(messages: &Value, (bot, group): (Id, Id), since: i64) -> (Vec<Posted>, bool) {
    let all = messages.as_array().map(Vec::as_slice).unwrap_or_default();
    let read: Vec<Posted> = all
        .iter()
        .filter_map(|message| {
            // 有的实现交回的不带这几格：照群消息的事件补上再读。
            let mut frame = message.clone();
            for (key, value) in [
                ("post_type", json!("message")),
                ("message_type", json!("group")),
                ("self_id", json!(bot)),
                ("group_id", json!(group)),
            ] {
                if frame.get(key).is_none_or(Value::is_null) {
                    frame[key] = value;
                }
            }
            match onebot::read(frame) {
                Frame::Event(Event::Group {
                    group: from,
                    posted,
                }) if from == group => Some(posted),
                _ => None,
            }
        })
        .collect();
    let earliest = read.iter().map(|posted| posted.time).min();
    let full = all.len() >= COUNT && earliest.is_some_and(|earliest| earliest > since);
    let mut found: Vec<Posted> = read
        .into_iter()
        .filter(|posted| posted.time >= since && posted.user != bot)
        .collect();
    found.sort_by_key(|posted| (posted.time, posted.message_id));
    (found, full)
}

#[cfg(test)]
mod tests;
