//! 会话的定时任务接进核心的定时（设计 12 第一节第 3 条、第三节第 4 到 6 条，施工 I-198）：
//!
//! - 页面的三个方法（[`Route::tasks_called`]）：`tasks.list {card}` 列这张卡的任务，`tasks.save {card, id?, text?, when?, paused?}`
//!   建一个（不带 `id`）或者改一个，`tasks.remove {card, id}` 删一个。卡只认群卡、私聊卡（写法同 `cards.save_one`）。只动自己建的、
//!   这张卡的那几个：核心别的定时、看一眼的定时、别的卡的任务不列、不改。核心拒了的（时间写得不对）把它那一句交回页面。
//! - 到点（[`Route::task_fired`]）：这张卡没了的删掉这个定时；卡片「是否回复」关着的不做、记一行（任务留着，下一次到点再看）；
//!   睡眠时间里照做（时刻是人定的）。在这张卡的会话里开一轮（没有会话的照发消息的办法找、造），只带一块事实 `scheduled-task`：
//!   人写的那段照模板转义。她说的话照常发到群里、私聊里。会话正有一轮在跑的，核心并进去、排在后面。
//! - 核心停着、桥没连着时错过的不补（核心报 `missed`，记一行）。

use miyu_chat::VenueKind;
use serde_json::{Value, json};

use super::judges::Tag;
use super::lanes::Responded;
use super::{Peer, Route, applied};
use crate::TARGET;
use crate::chat::{Id, To};
use crate::core::methods::{TASKS_LIST, TASKS_REMOVE, TASKS_SAVE};
use crate::core::{Gone, reason};
use crate::rules::{Card, card_edit};
use crate::tasks::Task;

/// 定时任务那一块事实的 `kind`。
const FACT: &str = "scheduled-task";

/// 定时任务开一轮的命令编号的开头（后面接定时的编号、该响的时刻）。
const OPENED: &str = "task/";

impl Route {
    /// 核心转来的一次 `method.call`（`pushed`）是不是定时任务的三个方法：是的照办、答，交回真。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn tasks_called(&mut self, pushed: &Value) -> Result<bool, Gone> {
        let id = pushed["id"].clone();
        let args = &pushed["params"]["params"];
        let method = pushed["params"]["method"].as_str();
        if !matches!(method, Some(TASKS_LIST | TASKS_SAVE | TASKS_REMOVE)) {
            return Ok(false);
        }
        let answer = match self.task_card(args) {
            None => None,
            Some(card) => match method {
                Some(TASKS_LIST) => Some(self.tasks_list(&card).await?),
                Some(TASKS_SAVE) => self.tasks_save(&card, args).await?,
                _ => self.tasks_remove(&card, args).await?,
            },
        };
        match answer {
            Some(answer) => self.core.answerer().answer(&id, answer),
            None => self.core.answerer().refuse(&id, "bad_params"),
        }
        Ok(true)
    }

    /// 参数 `args` 的 `card` 说的那张群卡、私聊卡，写成一份没有字的任务（只拿来比是不是同一张卡）；别的卡、写错的是空的。
    fn task_card(&self, args: &Value) -> Option<Task> {
        let platform = self.platforms.ours(args).ok().map(|one| one.name());
        let (scope, group, id) = match card_edit::card_of(&args["card"], platform)? {
            Card::Group(scope, id) => (scope, true, id),
            Card::Private(scope, id) => (scope, false, id),
            _ => return None,
        };
        let bot = scope.bot.as_ref().map(Id::as_str);
        Some(Task::new((&scope.platform, bot), (group, id.as_str()), ""))
    }

    /// 自己建的任务里卡 `card` 的那几个：核心列的那一项、读出来的任务。列不出的记一行、当没有。
    async fn own_tasks(&mut self, card: &Task) -> Result<Vec<(Value, Task)>, Gone> {
        let reply = self.core.call("schedule.list", json!({})).await?;
        if let Some(reason) = reason(&reply) {
            tracing::warn!(target: TARGET, reason, "tasks not listed");
            return Ok(Vec::new());
        }
        Ok(reply["result"]["schedules"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|one| one["target"]["package"].is_object())
            .filter_map(|one| {
                let task = one["prompt"].as_str().and_then(Task::read)?;
                task.same_card(card).then(|| (one.clone(), task))
            })
            .collect())
    }

    /// `tasks.list`：`{tasks: [{id, text, when, next?, last?, paused}]}`，照核心列的先后。
    async fn tasks_list(&mut self, card: &Task) -> Result<Value, Gone> {
        let tasks: Vec<Value> = self
            .own_tasks(card)
            .await?
            .into_iter()
            .map(|(one, task)| {
                json!({
                    "id": one["id"],
                    "text": task.text,
                    "when": one["when"],
                    "next": one["next"],
                    "last": one["last"],
                    "paused": one["paused"] == true,
                })
            })
            .collect();
        Ok(json!({ "tasks": tasks }))
    }

    /// `tasks.save`：不带 `id` 的建一个（要 `text`、`when`）；带的改那一个的字、时间、停不停（写了哪样改哪样）。答 `{saved: true,
    /// id}`；核心拒了的答 `{saved: false, message}`。参数写错、不是这张卡的任务的交回空的（拒 `bad_params`）。
    async fn tasks_save(&mut self, card: &Task, args: &Value) -> Result<Option<Value>, Gone> {
        let text = match &args["text"] {
            Value::Null => None,
            Value::String(text) if !text.trim().is_empty() => Some(text.trim()),
            _ => return Ok(None),
        };
        let when = args.get("when").filter(|when| when.is_object());
        let prompt = |text: &str| card.with_text(text).text();
        let Some(id) = args["id"].as_str() else {
            let (Some(text), Some(when)) = (text, when) else {
                return Ok(None);
            };
            let params = json!({"when": when, "prompt": prompt(text), "target": {"package": {}}});
            let reply = self.core.call("schedule.add", params).await?;
            return Ok(Some(saved(
                &reply,
                reply["result"]["schedule"]["id"].clone(),
            )));
        };
        if !self.owns_task(card, id).await? {
            return Ok(None);
        }
        if text.is_some() || when.is_some() {
            let mut params = json!({ "id": id });
            if let Some(text) = text {
                params["prompt"] = json!(prompt(text));
            }
            if let Some(when) = when {
                params["when"] = when.clone();
            }
            let reply = self.core.call("schedule.update", params).await?;
            if reason(&reply).is_some() {
                return Ok(Some(saved(&reply, json!(id))));
            }
        }
        if let Some(paused) = args["paused"].as_bool() {
            let method = if paused {
                "schedule.pause"
            } else {
                "schedule.resume"
            };
            let reply = self.core.call(method, json!({ "id": id })).await?;
            return Ok(Some(saved(&reply, json!(id))));
        }
        Ok(Some(json!({"saved": true, "id": id})))
    }

    /// `tasks.remove`：删这张卡的那一个，答 `{removed: true}`；不是这张卡的任务的交回空的。
    async fn tasks_remove(&mut self, card: &Task, args: &Value) -> Result<Option<Value>, Gone> {
        let Some(id) = args["id"].as_str() else {
            return Ok(None);
        };
        if !self.owns_task(card, id).await? {
            return Ok(None);
        }
        let reply = self
            .core
            .call("schedule.cancel", json!({ "id": id }))
            .await?;
        Ok(Some(json!({"removed": reason(&reply).is_none()})))
    }

    /// 定时 `id` 是不是卡 `card` 的任务。
    async fn owns_task(&mut self, card: &Task, id: &str) -> Result<bool, Gone> {
        let own = self.own_tasks(card).await?;
        Ok(own.iter().any(|(one, _)| one["id"] == id))
    }

    /// 核心推来 `schedule.fired`（`params`），`prompt` 读出来是任务 `task`：过一道，开一轮（模块的说明）。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn task_fired(&mut self, params: &Value, task: Task) -> Result<(), Gone> {
        let id = params["id"].as_str().unwrap_or_default().to_string();
        if let Some(missed) = params["missed"].as_u64() {
            tracing::info!(target: TARGET, schedule = %id, missed, "task runs missed");
        }
        let kind = if task.group {
            VenueKind::Group
        } else {
            VenueKind::Private
        };
        let made = self.platforms.named(&task.platform).and_then(|platform| {
            let number = Id::new(&task.id)?;
            let bot = self
                .platforms
                .sender(platform, task.bot.as_deref().and_then(Id::new))?;
            let venue = platform.venue(&bot, kind, &number).ok()?;
            Some((platform, bot, number, venue))
        });
        let Some((platform, bot, number, venue)) = made else {
            tracing::warn!(target: TARGET, schedule = %id, platform = %task.platform, "task skipped, no connection for its card");
            return Ok(());
        };
        let listed = match kind {
            VenueKind::Group => self.rules.cards().lists_group(&venue),
            VenueKind::Private => self.rules.cards().lists_private(&venue),
        };
        if !listed {
            tracing::info!(target: TARGET, schedule = %id, venue = %venue.id(), "task cancelled, its card is gone");
            self.core
                .call("schedule.cancel", json!({ "id": id }))
                .await?;
            return Ok(());
        }
        let rules = self.applied(&venue);
        if applied::allow(&rules) == Some(false) {
            tracing::info!(target: TARGET, schedule = %id, venue = %venue.id(), why = "off", "task skipped");
            return Ok(());
        }
        let to = match kind {
            VenueKind::Group => To::Group(number),
            VenueKind::Private => To::Private(number),
        };
        let found = self
            .target_session((platform, &venue), to.clone(), bot.clone())
            .await?;
        let Some(session) = found else {
            tracing::warn!(target: TARGET, schedule = %id, venue = %venue.id(), "task skipped, no session");
            return Ok(());
        };
        // 这个会话还没来过消息的（桥起来以后）：她说的话发到哪照这里记。
        self.peers.entry(session.clone()).or_insert(Peer {
            platform,
            bot,
            to,
            venue: venue.id().clone(),
        });
        let fields = [("text", task.text.as_str())];
        let facts = match self
            .rules
            .scheduled_task()
            .render(&fields.into_iter().collect())
        {
            Ok(text) => vec![json!({"kind": FACT, "text": text})],
            Err(error) => {
                tracing::warn!(target: TARGET, error = %error, "task fact not written");
                return Ok(());
            }
        };
        let at = params["at"].as_str().unwrap_or_default();
        let tag = Tag {
            id: format!("{OPENED}{id}/{at}"),
            number: 0,
            venue: venue.id().to_string(),
            who: String::new(),
            text: String::new(),
            facts: Vec::new(),
        };
        let asked = self.respond(&session, &[], &facts, &tag).await?;
        if asked == Responded::Asked {
            tracing::info!(target: TARGET, schedule = %id, venue = %venue.id(), "task opened");
        }
        Ok(())
    }
}

/// 核心的回应 `reply` 写成页面要的：成了的 `{saved: true, id}`，拒了的 `{saved: false, message}`（核心照连接的语言写的那一句）。
fn saved(reply: &Value, id: Value) -> Value {
    match reason(reply) {
        None => json!({"saved": true, "id": id}),
        Some(reason) => {
            let message = reply["error"]["message"].as_str().unwrap_or(reason);
            json!({"saved": false, "message": message})
        }
    }
}
