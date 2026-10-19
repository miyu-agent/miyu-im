//! 头像和群成员（施工 O-54，`onebot.md` 第一条「头像和群成员」，2026-10-17 项目主人定）：两件工具，读的一头把 `tool.call` 照推送
//! 交到这里，各起一个任务办（`chores`，同取图），跟核心的那一头不等。
//!
//! - `avatar`：一个人的（`user`）、一个群的（`group`）头像，照 `bridge.json` 的地址（`avatar_user_url`、`avatar_group_url`，
//!   `{id}` 换成号）下下来，存进这个会话的工作区 `qq-files/avatar-qq-<号>.<扩展名>`、`avatar-group-<号>.<…>`，答相对工作区的
//!   路径（同 `fetch_media`）；她看不看图、拿它做什么是她的事。两个都没写的取叫她的人（`by.id`；私聊里是对方）。
//! - `members`：这个群的成员（`get_group_member_list`），`query` 写了的只留名字（群名片、昵称）含着它的，最多列
//!   `members_shown` 个，一行一个：QQ 号、名字、身份。

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use super::fields::{NAME, clean};
use super::{Peer, Route};
use crate::TARGET;
use crate::chat::Bots;
use crate::chat::Source;
use crate::chat::{CallError, Member, Rank, To};
use crate::core::{Answerer, Gone};
use crate::media::{QQ_FILES, bytes, extension, save};
use crate::rules::{AVATAR, Tools};

/// 答的话里的原因最多几个字符（同取图）。
const DETAIL: usize = 200;

/// 要谁的头像（纯逻辑）：她给的群号、QQ 号（正整数，写成字的也认），都没给的取叫她的人。两个都给了的、写错的是空的。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Whose {
    /// 一个人。
    User(i64),
    /// 一个群。
    Group(i64),
}

/// 读 `avatar` 的参数 `args`，叫她的人是 `caller`（号；本机的、不知道的是空的）。
pub(super) fn whose(args: &Value, caller: Option<i64>) -> Option<Whose> {
    let number = |value: &Value| match value {
        Value::Null => Some(None),
        Value::Number(number) => number.as_i64().filter(|n| *n > 0).map(Some),
        Value::String(text) => text.trim().parse::<i64>().ok().filter(|n| *n > 0).map(Some),
        _ => None,
    };
    match (number(&args["user"])?, number(&args["group"])?) {
        (Some(user), None) => Some(Whose::User(user)),
        (None, Some(group)) => Some(Whose::Group(group)),
        (None, None) => caller.map(Whose::User),
        (Some(_), Some(_)) => None,
    }
}

/// 头像的地址和存的文件名（不带扩展名）：照模板 `(一个人的, 一个群的)` 换号。
pub(super) fn avatar_of(whose: Whose, templates: &(String, String)) -> (String, String) {
    match whose {
        Whose::User(user) => (
            templates.0.replace("{id}", &user.to_string()),
            format!("avatar-qq-{user}"),
        ),
        Whose::Group(group) => (
            templates.1.replace("{id}", &group.to_string()),
            format!("avatar-group-{group}"),
        ),
    }
}

/// 群成员列表 `data`（`get_group_member_list` 的回答）照 `query` 筛、列成一行（纯逻辑）：交回（列的几个、一共几个、那一行）。
/// 名字照群名片，空的照昵称（洗过）；`query` 照名字、号找，不分大小写；身份没写的当 `member`；一个一段 `<号> <名字> (<身份>)`，
/// `; ` 隔开；最多 `shown` 个。
pub(super) fn listed(
    members: &[Member],
    query: Option<&str>,
    shown: usize,
) -> (usize, usize, String) {
    let query = query
        .map(|query| query.trim().to_lowercase())
        .filter(|query| !query.is_empty());
    let found: Vec<String> = members
        .iter()
        .filter_map(|member| {
            let user = member.user;
            let name = member
                .name
                .as_deref()
                .and_then(|name| clean(name, NAME))
                .unwrap_or_default();
            if let Some(query) = &query
                && !name.to_lowercase().contains(query.as_str())
                && !user.to_string().contains(query.as_str())
            {
                return None;
            }
            let role = member.rank.map_or("member", Rank::word);
            Some(format!("{user} {name} ({role})"))
        })
        .collect();
    let total = found.len();
    let list: Vec<String> = found.into_iter().take(shown).collect();
    (list.len(), total, list.join("; "))
}

/// 交给任务办的一次。
struct Job {
    /// `tool.call` 的编号。
    id: Value,
    /// 工具名。
    tool: String,
    /// 发到哪：机器人号、场所。
    peer: Peer,
    /// 要做的。
    work: Work,
    /// 机器人号的连接。
    bots: Arc<Bots>,
    /// 答的话。
    tools: Arc<Tools>,
    /// 往核心写回应的一头。
    answerer: Answerer,
}

/// 要做的一件。
enum Work {
    /// 下头像：地址、存的文件名（不带扩展名）、工作区、家目录、最多等多久。
    Avatar {
        url: String,
        name: String,
        cwd: String,
        home: Option<std::path::PathBuf>,
        wait: Duration,
    },
    /// 列群成员：群号、找什么、最多列几个。
    Members {
        group: i64,
        query: Option<String>,
        shown: usize,
    },
}

impl Route {
    /// 核心转来的一次 `avatar`、`members`（`pushed` 是 `tool.call`）：认参数，起一个任务办。桥不认识这个会话的答 `unreachable`。
    ///
    /// # Errors
    ///
    /// 收这个会话留着的推送时写不出去、核心断开。
    pub(super) async fn looked_up(&mut self, pushed: &Value) -> Result<(), Gone> {
        let params = &pushed["params"];
        let (id, tool) = (&pushed["id"], params["tool"].as_str().unwrap_or_default());
        let line = params["session"].as_str().unwrap_or_default().to_string();
        let session = self.main_of(&line);
        let answerer = self.core.answerer();
        self.gather(&session).await?;
        let Some(peer) = self.peers.get(&session).cloned() else {
            tracing::warn!(target: TARGET, session, tool, why = "unreachable", "platform tool failed");
            answerer.answer(id, self.tools.answer("unreachable", &[], true));
            return Ok(());
        };
        let work = if tool == AVATAR {
            let caller = match peer.to {
                To::Private(user) => Some(user),
                To::Group(_) => caller_number(params),
            };
            let Some(whose) = whose(&params["args"], caller) else {
                tracing::info!(target: TARGET, venue = %peer.venue, tool, why = "no-target", "platform tool refused");
                answerer.answer(id, self.tools.answer("no-target", &[], true));
                return Ok(());
            };
            let (url, name) = avatar_of(whose, &self.fetching.avatars);
            Work::Avatar {
                url,
                name,
                cwd: params["cwd"].as_str().unwrap_or_default().to_string(),
                home: std::env::home_dir(),
                wait: self.fetching.wait,
            }
        } else {
            let To::Group(group) = peer.to else {
                answerer.answer(id, self.tools.call(tool));
                return Ok(());
            };
            Work::Members {
                group,
                query: params["args"]["query"].as_str().map(str::to_string),
                shown: self.fetching.members,
            }
        };
        let job = Job {
            id: id.clone(),
            tool: tool.to_string(),
            peer,
            work,
            bots: Arc::clone(&self.bots),
            tools: Arc::clone(&self.tools),
            answerer,
        };
        self.chores.spawn(run(job));
        Ok(())
    }
}

/// 叫她的人的号：`tool.call` 的 `by` 是外部身份的照 `by.id`，私聊里认出的终端管理员照 `by.via`；别的是空的。
fn caller_number(params: &Value) -> Option<i64> {
    let by = &params["by"];
    let id = match by["kind"].as_str() {
        Some("external") => by["id"].as_str(),
        Some("person") => by["via"].as_str(),
        _ => None,
    }?;
    id.strip_prefix("qq:")?.parse().ok()
}

/// 办一次，答核心、记运行日志。
async fn run(job: Job) {
    let venue = job.peer.venue.as_str().to_string();
    let answered = match &job.work {
        Work::Avatar {
            url,
            name,
            cwd,
            home,
            wait,
        } => avatar(url, name, (cwd, home.clone()), *wait)
            .await
            .map(|path| job.tools.answer("saved", &[("path", &path)], false)),
        Work::Members {
            group,
            query,
            shown,
        } => members(&job, *group, query.as_deref(), *shown).await,
    };
    match answered {
        Ok(answer) => {
            tracing::info!(target: TARGET, venue, tool = job.tool, "platform tool done");
            job.answerer.answer(&job.id, answer);
        }
        Err((why, detail)) => {
            tracing::warn!(target: TARGET, venue, tool = job.tool, why, "platform tool failed");
            let fields: Vec<(&str, &str)> = detail
                .as_deref()
                .map(|detail| ("detail", detail))
                .into_iter()
                .collect();
            job.answerer
                .answer(&job.id, job.tools.answer(why, &fields, true));
        }
    }
}

/// 没做成的：那一句的名字、带的原因。
type Undone = (&'static str, Option<String>);

/// 下头像、存进工作区：交回相对工作区的路径。
async fn avatar(
    url: &str,
    name: &str,
    (cwd, home): (&str, Option<std::path::PathBuf>),
    wait: Duration,
) -> Result<String, Undone> {
    let got = bytes(&[Source::Url(url.to_string())], wait)
        .await
        .map_err(|why| ("unfetched", Some(cut(&why))))?;
    let file = format!("{name}.{}", extension(&got));
    let cwd = cwd.to_string();
    let saved = tokio::task::spawn_blocking(move || save(&cwd, home.as_deref(), &file, &got))
        .await
        .map_err(|error| error.to_string())
        .and_then(|saved| saved);
    match saved {
        Ok(path) => {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            Ok(format!("{QQ_FILES}/{}", name.unwrap_or_default()))
        }
        Err(detail) => Err(("not-saved", Some(cut(&detail)))),
    }
}

/// 列群成员。
async fn members(
    job: &Job,
    group: i64,
    query: Option<&str>,
    shown: usize,
) -> Result<Value, Undone> {
    let Some(link) = job.bots.get(job.peer.bot) else {
        return Err(("unreachable", None));
    };
    let list = link.members(group).await.map_err(|error| match error {
        CallError::Failed { said, .. } => ("failed", Some(said)),
        CallError::Timeout => ("unanswered", None),
        CallError::Closed => ("unreachable", None),
    })?;
    let (count, total, list) = listed(&list, query, shown);
    if count == 0 {
        return Ok(job.tools.answer("no-members", &[], false));
    }
    let (count, total) = (count.to_string(), total.to_string());
    Ok(job.tools.answer(
        "members",
        &[("shown", &count), ("total", &total), ("list", &list)],
        false,
    ))
}

/// 截到 [`DETAIL`] 个字符。
fn cut(text: &str) -> String {
    text.chars().take(DETAIL).collect()
}

#[cfg(test)]
mod tests;
