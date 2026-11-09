//! 发一段字（施工 I-183 中下，设计 07 第 3.5 节）：
//!
//! - 照调的先后排队发（宿主的出站队列要先后），一个机器人一条队、一个任务。
//! - 同一个聊天两条之间至少隔 [`GAP`]（Telegram：同一个聊天最好一秒不超过一条）；回了 429 照 `retry_after` 等了再发，不硬发。
//! - 一条最多 [`LIMIT`] 个 UTF-16 单位（不是字数：emoji 多了照字数切会超，AstrBot 踩过），超了拆成几条，照换行、空白、标点找断开的
//!   地方；第一条带引用、@，后面几条不带。发出去的编号交最后一条的。
//! - 纯文本，不转 MarkdownV2。
//! - 回在聊天最近那条消息的话题里（[`Known::thread`]）。
//! - @ 人：有用户名的写 `@用户名`（Telegram 自己认），没有的写名字、挂一个 `text_mention`（UTF-16 的偏移）。
//! - 图、文件（施工 I-185 中，[`crate::media`]）排在同一条队里，照 `multipart/form-data` 传，图的说明带 @。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;

use miyu_im::chat::{CallError, DETAIL, Lead};

use crate::api::{Api, Failure};
use crate::known::Known;
use crate::media::{Upload, form};

/// 一条最多多少个 UTF-16 单位（Bot API 的 `sendMessage`）。
pub const LIMIT: usize = 4096;

/// 同一个聊天两条之间至少隔多久。
pub const GAP: Duration = Duration::from_secs(1);

/// 一次调用最多等多久。
const CALL: Duration = Duration::from_secs(30);

/// 传一份图、文件最多等多久（最大 50 MB）。
const UPLOAD: Duration = Duration::from_secs(120);

/// 429 最多照着等几次，再不行算没成。
const RETRIES: usize = 3;

/// 发的是什么。
pub enum Body {
    /// 几段字，每段 ≤ [`LIMIT`]：字和它的实体（第一段带 @）。
    Text(Vec<(String, Vec<Value>)>),
    /// 一份图、文件（施工 I-185 中）。
    File(Upload),
}

/// 排着的一件：发到哪个聊天、发什么、引用、答到哪。
pub struct Job {
    /// 聊天的号。
    pub chat: String,
    /// 发什么。
    pub body: Body,
    /// 引用的那一条的编号（只第一段带）。
    pub reply: Option<i64>,
    /// 交答：发出去的最后一条的编号。
    pub answer: oneshot::Sender<Result<Option<String>, CallError>>,
}

/// 排一件的口子：放进去就算交出去了，等的另一头拿答。
pub type Queue = mpsc::UnboundedSender<Job>;

/// 起一条队：交回口子和发的那个任务（放下口子任务就停）。
pub fn queue(api: Api, known: Arc<Known>) -> (Queue, impl std::future::Future<Output = ()> + Send) {
    let (queue, mut jobs) = mpsc::unbounded_channel::<Job>();
    let writer = async move {
        let mut last: HashMap<String, Instant> = HashMap::new();
        while let Some(job) = jobs.recv().await {
            let result = deliver(&api, &known, &job, &mut last).await;
            if job.answer.send(result).is_err() {
                // 等的一头不等了：不要紧。
            }
        }
    };
    (queue, writer)
}

/// 发一件（几段字的、一份图或文件的）：交回最后一条的编号。
async fn deliver(
    api: &Api,
    known: &Known,
    job: &Job,
    last: &mut HashMap<String, Instant>,
) -> Result<Option<String>, CallError> {
    let count = match &job.body {
        Body::Text(parts) => parts.len(),
        Body::File(_) => 1,
    };
    let mut sent = None;
    for index in 0..count {
        if let Some(at) = last.get(&job.chat) {
            tokio::time::sleep_until(*at + GAP).await;
        }
        let mut params = json!({"chat_id": job.chat});
        if let Some(thread) = known.thread(&job.chat) {
            params["message_thread_id"] = json!(thread);
        }
        if index == 0
            && let Some(reply) = job.reply
        {
            // 引用的那一条撤了的照样发，不带引用。
            params["reply_parameters"] =
                json!({"message_id": reply, "allow_sending_without_reply": true});
        }
        let reply = match &job.body {
            Body::Text(parts) => {
                let (text, entities) = &parts[index];
                params["text"] = json!(text);
                if !entities.is_empty() {
                    params["entities"] = json!(entities);
                }
                call(api, Request::Json(&params)).await
            }
            Body::File(upload) => {
                if let Some((caption, entities)) = &upload.caption {
                    params["caption"] = json!(caption);
                    if !entities.is_empty() {
                        params["caption_entities"] = json!(entities);
                    }
                }
                let fields = params.as_object().cloned().unwrap_or_default();
                call(api, Request::Form(&fields, upload)).await
            }
        };
        last.insert(job.chat.clone(), Instant::now());
        sent = reply?["message_id"].as_i64().map(|id| id.to_string());
    }
    Ok(sent)
}

/// 一次调用：发字的 `sendMessage`，传图、文件的照 [`Upload::method`]。
enum Request<'a> {
    /// `sendMessage` 的参数。
    Json(&'a Value),
    /// 几格字和一份要传的。
    Form(&'a serde_json::Map<String, Value>, &'a Upload),
}

/// 调一次：429 照它说的等了再发，最多 [`RETRIES`] 次。
async fn call(api: &Api, request: Request<'_>) -> Result<Value, CallError> {
    let mut tried = 0;
    loop {
        let answered = match request {
            Request::Json(params) => api.call("sendMessage", params, CALL).await,
            Request::Form(fields, upload) => {
                let (content_type, body) = form(fields, upload);
                api.post_form(upload.method, (&content_type, body), UPLOAD)
                    .await
            }
        };
        match answered {
            Ok(sent) => return Ok(sent),
            Err(Failure::Retry(wait)) if tried < RETRIES => {
                tried += 1;
                tokio::time::sleep(wait).await;
            }
            Err(Failure::Failed { code, description }) => {
                let said: String = format!("{code} {description}")
                    .chars()
                    .take(DETAIL)
                    .collect();
                return Err(CallError::Failed {
                    said,
                    message: description,
                });
            }
            Err(Failure::Network(reason)) if reason.starts_with("timeout") => {
                return Err(CallError::Timeout);
            }
            Err(_) => return Err(CallError::Closed),
        }
    }
}

/// 一段话前面带上 `lead` 的 @，拆成几段（每段 ≤ `limit` 个 UTF-16 单位）：第一段带 @ 的实体。
pub fn parts(text: &str, lead: &Lead, known: &Known, limit: usize) -> Vec<(String, Vec<Value>)> {
    let mut prefix = String::new();
    let mut entities = Vec::new();
    let mut targets: Vec<&str> = lead.at.iter().map(String::as_str).collect();
    for more in &lead.more {
        if !targets.contains(&more.as_str()) {
            targets.push(more);
        }
    }
    for user in targets {
        let person = known.person(user);
        match person
            .as_ref()
            .and_then(|person| person.username.as_deref())
        {
            Some(username) => prefix.push_str(&format!("@{username} ")),
            None => {
                let shown = person.map_or_else(|| user.to_string(), |person| person.name);
                let shown = if shown.is_empty() {
                    user.to_string()
                } else {
                    shown
                };
                let offset = prefix.encode_utf16().count();
                let label = format!("@{shown}");
                entities.push(json!({"type": "text_mention", "offset": offset, "length": label.encode_utf16().count(), "user": {"id": number(user)}}));
                prefix.push_str(&label);
                prefix.push(' ');
            }
        }
    }
    let whole = format!("{prefix}{text}");
    let mut parts: Vec<(String, Vec<Value>)> = split(&whole, limit)
        .into_iter()
        .map(|part| (part, Vec::new()))
        .collect();
    if let Some(first) = parts.first_mut() {
        // @ 都在开头，拆不开（`split` 不在开头的那一小段里断）。
        first.1 = entities;
    }
    parts
}

/// 号写成数（Telegram 的 `user.id` 是整数）；读不出的照字交，Telegram 自己回没成。
fn number(user: &str) -> Value {
    user.parse::<i64>()
        .map_or_else(|_| json!(user), |id| json!(id))
}

/// 拆成每段 ≤ `limit` 个 UTF-16 单位：在 `limit` 以内最后一个换行处断，没有换行的找空白、句读，都没有的硬断（不切开一个字）。
pub fn split(text: &str, limit: usize) -> Vec<String> {
    let mut parts = Vec::new();
    let mut rest = text;
    while rest.encode_utf16().count() > limit {
        let mut end = 0;
        let mut units = 0;
        let mut breaks = (None, None);
        for (at, ch) in rest.char_indices() {
            units += ch.len_utf16();
            if units > limit {
                break;
            }
            end = at + ch.len_utf8();
            if ch == '\n' {
                breaks.0 = Some(end);
            } else if ch.is_whitespace() || "。！？；，、.!?;,".contains(ch) {
                breaks.1 = Some(end);
            }
        }
        let cut = breaks
            .0
            .or(breaks.1)
            .filter(|cut| *cut > 0)
            .unwrap_or(end.max(rest.chars().next().map_or(1, char::len_utf8)));
        let (head, tail) = rest.split_at(cut);
        let head = head.trim_end();
        if !head.is_empty() {
            parts.push(head.to_string());
        }
        rest = tail.trim_start_matches('\n');
    }
    if !rest.trim().is_empty() || parts.is_empty() {
        parts.push(rest.to_string());
    }
    parts
}

#[cfg(test)]
mod tests;
