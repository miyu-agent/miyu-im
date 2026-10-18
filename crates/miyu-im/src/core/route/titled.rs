//! 群名、群号（施工 O-49，`onebot.md` 第一条「群名」）：她在群里知道自己在哪个群。群会话找到、消息记下以后问 NapCat 的
//! `get_group_info`，照出厂的模板（`system/group.txt`，给模型看的字，登记在 `docs/ledger.md`）写一段，经核心的
//! `system.append {content, session}` 放进这个会话的 system 尾部（核心的 `extensions.md`「放在 system 尾部的」）。
//!
//! - 放在 system：一直在、压缩不掉；群名很少改，改了才换一次（system 一换，这个会话前面的缓存就作废）。群名是群里的人改得了的字，
//!   照模板的规矩转义（`Template::render`），截到 64 个字符，裹在标签里只当名字看。
//! - 问到了隔 `bridge.json` 的 `group_info_seconds` 再问；问不到的（没连着、失败、名字是空的）这一次不放，下一条消息照样隔这么久
//!   再问：NapCat 一卡，不能每条消息都等一个时限。写的和上一次放的一样的不再交（核心照快照比，交了也不换）。
//! - 支线（施工 O-36）是另一个会话：分叉以后照主线放的那一段给它也放一份，支线第一次请求的 system 和主线的一样。
//! - 附加提示词（施工 O-58 上，「场所卡片」）：场所规则的 `extra_prompt` 照模板 `system/note.txt` 写一段，和群名那一段拼成一段
//!   交（核心照扩展、照会话只留一层，后交的整段替换前面的）。每一条消息进来都照这一刻的规则放：群里判以前放好，私聊交以前
//!   先找到会话放好，这一条开的轮就带上；卡片改了，下一条消息照新的。规则删了提示词、群名也没有的交 `null` 去掉那一层。

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use serde_json::json;

use super::Route;
use super::fields::{NAME, clean};
use crate::TARGET;
use crate::core::{Gone, reason};

/// 附加提示词的模板先填这一段占位，再换成原文（施工 O-58 上）：只有字母和下划线，模板的填法不会改它。
const SLOT: &str = "VENUE_NOTE_TEXT";

/// 问过的群名、放过的那一段：按会话记。
#[derive(Debug)]
pub(super) struct Titles {
    /// 问到了隔多久再问。
    every: Duration,
    /// 会话 → 上一次问的时刻。
    asked: HashMap<String, Instant>,
    /// 会话 → 问到的群名那一段（照模板写好的）。
    titles: HashMap<String, String>,
    /// 会话 → 附加提示词那一段（照模板写好的，施工 O-58 上）。
    notes: HashMap<String, String>,
    /// 会话 → 上一次放进 system 的那一段。
    set: HashMap<String, String>,
}

impl Titles {
    /// 一个空的：问到了隔 `every` 再问。
    pub(super) fn new(every: Duration) -> Titles {
        Titles {
            every,
            asked: HashMap::new(),
            titles: HashMap::new(),
            notes: HashMap::new(),
            set: HashMap::new(),
        }
    }

    /// 会话 `session` 此刻 `now` 该不该问：没问过的、上一次问到此刻不止 `every` 的该问，记下这一次。
    fn due(&mut self, session: &str, now: Instant) -> bool {
        let due = self
            .asked
            .get(session)
            .is_none_or(|at| now.saturating_duration_since(*at) >= self.every);
        if due {
            self.asked.insert(session.to_string(), now);
        }
        due
    }

    /// 会话 `session` 该放的一整段：群名那一段、附加提示词那一段，去掉末尾的空白、一行接一行；都没有的是空的。
    fn tail(&self, session: &str) -> Option<String> {
        let parts: Vec<&str> = [self.titles.get(session), self.notes.get(session)]
            .into_iter()
            .flatten()
            .map(|part| part.trim_end())
            .collect();
        (!parts.is_empty()).then(|| parts.join("\n"))
    }
}

/// 平台给的群名洗过（同发的人的名字），洗完空的是空的。
pub(super) fn group_name(name: &str) -> Option<String> {
    clean(name, NAME)
}

impl Route {
    /// 群号是 `group` 的群会话 `session`（收进它的机器人号 `bot`）：该问的问群名，名字换了的照模板写一段放进它的 system 尾部。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn titled(&mut self, session: &str, group: i64, bot: i64) -> Result<(), Gone> {
        if !self.titles.due(session, Instant::now()) {
            return Ok(());
        }
        let asked = match self.bots.get(bot) {
            Some(link) => link
                .group_name(group)
                .await
                .map_err(|error| format!("{error:?}")),
            None => Err("bot not connected".to_string()),
        };
        let Some(name) = asked
            .as_ref()
            .ok()
            .and_then(|name| name.as_deref().and_then(group_name))
        else {
            let error = asked.err().unwrap_or_default();
            tracing::debug!(target: TARGET, group, error, "group name unknown");
            return Ok(());
        };
        let id = group.to_string();
        let fields = BTreeMap::from([("name", name.as_str()), ("id", id.as_str())]);
        let content = match self.rules.group_title().render(&fields) {
            Ok(content) => content,
            Err(error) => {
                // 照说不会：起来时查过模板只要这两个字段（`rules/facts.rs`）。
                tracing::warn!(target: TARGET, group, error = %error, "group title not written");
                return Ok(());
            }
        };
        self.titles.titles.insert(session.to_string(), content);
        self.append_tail(session).await
    }

    /// 会话 `session` 的附加提示词换成 `note`（场所规则的 `extra_prompt`，没设的是空的，施工 O-58 上）：照模板写好，整段变了的
    /// 再交。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn noted(&mut self, session: &str, note: Option<&str>) -> Result<(), Gone> {
        // 外壳照模板，原文原样换进去：模板的填法会把换行这些转义掉，那是给群名这种别人改得了的字的；附加提示词是管理员在卡片
        // 里自己写的几行字。
        let rendered = note.and_then(|text| {
            let fields = BTreeMap::from([("text", SLOT)]);
            self.rules
                .note()
                .render(&fields)
                .inspect_err(|error| {
                    // 照说不会：起来时查过模板只要这一个字段（`rules/facts.rs`）。
                    tracing::warn!(target: TARGET, session, error = %error, "venue note not written");
                })
                .ok()
                .map(|shell| shell.replacen(SLOT, text, 1))
        });
        match rendered {
            Some(rendered) => self.titles.notes.insert(session.to_string(), rendered),
            None => self.titles.notes.remove(session),
        };
        self.append_tail(session).await
    }

    /// 支线 `lane` 分叉以后：主线 `main` 放过的那一段给它也放一份（同主线的 system）。主线还没放过的什么都不做。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn title_lane(&mut self, main: &str, lane: &str) -> Result<(), Gone> {
        match self.titles.set.get(main).cloned() {
            Some(content) => self.append_system(lane, Some(content)).await,
            None => Ok(()),
        }
    }

    /// 会话 `session` 拼好的一整段交上去：都没有了的交 `null` 去掉那一层（放过才交）。
    async fn append_tail(&mut self, session: &str) -> Result<(), Gone> {
        let tail = self.titles.tail(session);
        if tail.is_none() && !self.titles.set.contains_key(session) {
            return Ok(());
        }
        self.append_system(session, tail).await
    }

    /// 把 `content` 经 `system.append` 放进会话 `session` 的 system 尾部（`null` 去掉那一层）：和上一次放的一样的不交。核心拒了的
    /// 记一行，不再试（下一次该放时照样再交）。
    async fn append_system(&mut self, session: &str, content: Option<String>) -> Result<(), Gone> {
        if self.titles.set.get(session) == content.as_ref() {
            return Ok(());
        }
        let params = json!({"content": content, "session": session});
        let reply = self.core.call("system.append", params).await?;
        match reason(&reply) {
            None => {
                tracing::info!(target: TARGET, session, cleared = content.is_none(), "venue tail set");
                match content {
                    Some(content) => self.titles.set.insert(session.to_string(), content),
                    None => self.titles.set.remove(session),
                };
            }
            Some(reason) => {
                tracing::warn!(target: TARGET, session, reason, "venue tail not set");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
