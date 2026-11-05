//! `miyu im logs watch [会话]`（设计 10，施工 I-160 下）：实时看 AI 在一个会话（不写的是全部）里干嘛，一直印到 Ctrl+C。
//!
//! 1. 照 `miyu im web` 连已经在跑的核心、找 im 页面服务的端口、要一张交接票；im 没在跑的说一句「先 `miyu im start`」。
//! 2. 本机用票换会话号（`POST /login/ticket`），开页面通道（`/channel`）出示会话号。
//! 3. 认会话（[`resolve`]）：`qq:号` 照 im 见过的场所认群还是私聊，写全的 `qq:group:号`、`qq:private:号` 照原样。
//! 4. `watch.start` 先印最近留着的，再一条条印推来的。写着的思考、正文接在同一行后面（[`Printer`]）。
//!
//! 字照命令行的字表说（`watch/render`）；看全部的，每行前面带会话名。通道断了说一句，退出码 1。

use std::io::Write;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use miyu_client::open::Core;
use miyu_client::places::DataRoot;

use super::Item;
use super::hub::Batch;
use super::render::shown;
use crate::texts::Texts;
use crate::web::page_port;
use crate::{PACKAGE, PROGRAM, status_file};

/// 没办成的退出码。
const FAILED: u8 = 1;

type Channel = WebSocketStream<TcpStream>;

/// 跟着看：数据根 `root`、会话 `venue`（空的是全部），照 `texts` 说，行印在 `out`，出错印在 `err`。交回退出码（Ctrl+C 不交回）。
pub async fn follow(
    root: &DataRoot,
    venue: Option<&str>,
    texts: &Texts,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let failed = |err: &mut dyn Write, said: String| {
        say(err, &texts.error(&said));
        FAILED
    };
    let mut core = match Core::connect_running(root, PROGRAM).await {
        Ok(core) => core,
        Err(refused) if refused.reason == "unreachable" => {
            return failed(err, texts.web_not_running());
        }
        Err(refused) => return failed(err, refused.to_string()),
    };
    let status = match core.call(PACKAGE, "extension.status", json!({})).await {
        Ok(status) => status,
        Err(refused) => return failed(err, refused.to_string()),
    };
    let Some(port) = page_port(&status, status_file::read(root).as_ref()) else {
        return failed(err, texts.web_not_running());
    };
    let ticket = match core.call(PACKAGE, "login.ticket", json!({})).await {
        Ok(ticket) => ticket["ticket"].as_str().unwrap_or_default().to_string(),
        Err(refused) => return failed(err, refused.to_string()),
    };
    let mut channel = match open(port, &ticket).await {
        Ok(channel) => channel,
        Err(why) => return failed(err, texts.say("watch/cli-failed", &[("why", why)])),
    };
    let known = ask(&mut channel, 1, "watch.venues", json!({}))
        .await
        .unwrap_or_default();
    let venues: Vec<(String, String)> = known["venues"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|one| {
            Some((
                one["venue"].as_str()?.to_string(),
                one["name"].as_str().unwrap_or_default().to_string(),
            ))
        })
        .collect();
    let picked = match venue.map(|venue| resolve(venue, &venues)) {
        None => None,
        Some(Ok(picked)) => Some(picked),
        Some(Err(Unresolved::Unknown)) => {
            let seen = venues
                .iter()
                .map(|(venue, name)| format!("{venue}（{name}）"))
                .collect::<Vec<_>>()
                .join("、");
            let key = if seen.is_empty() {
                "watch/cli-none"
            } else {
                "watch/cli-unknown"
            };
            return failed(
                err,
                texts.say(
                    key,
                    &[
                        ("venue", venue.unwrap_or_default().to_string()),
                        ("seen", seen),
                    ],
                ),
            );
        }
        Some(Err(Unresolved::Both(id))) => {
            return failed(err, texts.say("watch/cli-both", &[("id", id)]));
        }
    };
    let started = match ask(&mut channel, 2, "watch.start", json!({"venue": picked})).await {
        Some(started) => started,
        None => return failed(err, texts.say("watch/cli-closed", &[])),
    };
    let shown_name = picked.as_deref().map(|picked| {
        venues
            .iter()
            .find(|(venue, _)| venue == picked)
            .map_or(picked.to_string(), |(_, name)| name.clone())
    });
    let heading = match &shown_name {
        Some(name) => texts.say("watch/cli-watching", &[("name", name.clone())]),
        None => texts.say("watch/cli-watching-all", &[]),
    };
    say(out, &heading);
    let offset = offset_minutes();
    let mut printer = Printer {
        all: picked.is_none(),
        ..Printer::default()
    };
    for batch in batches(&started["batches"]) {
        printer.print(&batch, offset, texts, out);
    }
    loop {
        let Some(frame) = next(&mut channel).await else {
            printer.end(out);
            return failed(err, texts.say("watch/cli-closed", &[]));
        };
        if frame["event"] != "watch" {
            continue;
        }
        if frame["reset"] == true {
            printer.end(out);
        }
        let incoming = match frame.get("batch") {
            Some(batch) => serde_json::from_value::<Batch>(batch.clone())
                .into_iter()
                .collect(),
            None => batches(&frame["batches"]),
        };
        for batch in incoming {
            printer.print(&batch, offset, texts, out);
        }
    }
}

/// 认不出的会话。
#[derive(Debug, PartialEq, Eq)]
pub enum Unresolved {
    /// 没见过。
    Unknown,
    /// 这个号既是群又是私聊：号。
    Both(String),
}

/// 认会话：写全的（`qq:group:号`、`qq:private:号`）照原样；`qq:号` 照见过的场所 `venues`（短编号、名字）认，群、私聊只有一边见过
/// 的就是它，两边都见过的、都没见过的认不出（纯逻辑）。
///
/// # Errors
///
/// 认不出的：没见过（[`Unresolved::Unknown`]）、两边都见过（[`Unresolved::Both`]）。
pub fn resolve(written: &str, venues: &[(String, String)]) -> Result<String, Unresolved> {
    let parts: Vec<&str> = written.split(':').collect();
    if let [_, "group" | "private", _] = parts.as_slice() {
        return Ok(written.to_string());
    }
    let [platform, id] = parts.as_slice() else {
        return Err(Unresolved::Unknown);
    };
    let found: Vec<&String> = ["group", "private"]
        .iter()
        .map(|kind| format!("{platform}:{kind}:{id}"))
        .filter_map(|candidate| {
            venues
                .iter()
                .find(|(venue, _)| *venue == candidate)
                .map(|(venue, _)| venue)
        })
        .collect();
    match found.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(Unresolved::Unknown),
        _ => Err(Unresolved::Both((*id).to_string())),
    }
}

/// 印行的那一头：看全部的带会话名；写着的块接在同一行后面，换了别的行先换行。写完的块边写边印过的（[`Printer::streamed`]）
/// 不再整行印：一轮里思考、调工具、正文先后写，写完的那一条一起来。
#[derive(Default)]
pub struct Printer {
    /// 看全部的：每行带会话名。
    pub all: bool,
    /// 正在接着印的块的编号。
    pub streaming: Option<String>,
    /// 边写边印过的块（最近 [`STREAMED`] 个）。
    pub streamed: std::collections::VecDeque<String>,
}

/// 记最近几个边写边印过的块。
pub const STREAMED: usize = 32;

impl Printer {
    /// 印一批。
    pub fn print(&mut self, batch: &Batch, offset: i32, texts: &Texts, out: &mut dyn Write) {
        let tool = |_: &str| None;
        for line in &batch.lines {
            let said = shown(line, offset, texts, &tool);
            let lead = if self.all {
                format!("{} [{}] ", said.at, name_of(batch))
            } else {
                format!("{} ", said.at)
            };
            match (&line.item, said.id) {
                (Item::Appended { .. }, Some(id)) => {
                    if self.streaming.as_deref() != Some(id.as_str()) {
                        self.end(out);
                        write(out, &format!("{lead}… "));
                        self.streaming = Some(id);
                    }
                    write(out, &said.text);
                }
                (Item::Wrote { done: false, .. }, Some(id)) => {
                    self.end(out);
                    write(out, &format!("{lead}{}", said.text));
                    self.streamed.push_back(id.clone());
                    if self.streamed.len() > STREAMED {
                        self.streamed.pop_front();
                    }
                    self.streaming = Some(id);
                }
                // 写完的块边写边印过：是正在接着印的那一行就换行收尾，别的不再印。没印过的整行印。
                (Item::Wrote { done: true, .. }, Some(id)) if self.streamed.contains(&id) => {
                    if self.streaming.as_deref() == Some(id.as_str()) {
                        self.end(out);
                    }
                }
                _ => {
                    self.end(out);
                    say(out, &format!("{lead}{}", said.text));
                }
            }
        }
    }

    /// 接着印的那一行收尾：换行。
    pub fn end(&mut self, out: &mut dyn Write) {
        if self.streaming.take().is_some() {
            say(out, "");
        }
    }
}

/// 一批的会话名：没名字的写短编号。
fn name_of(batch: &Batch) -> &str {
    if batch.name.is_empty() {
        &batch.venue
    } else {
        &batch.name
    }
}

/// 回应里的几批。
fn batches(value: &Value) -> Vec<Batch> {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

/// 用票换会话号、开通道、出示会话号。
async fn open(port: u16, ticket: &str) -> Result<Channel, String> {
    let body = json!({"ticket": ticket}).to_string();
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|error| error.to_string())?;
    let head = format!(
        "POST /login/ticket HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .await
        .map_err(|error| error.to_string())?;
    stream
        .write_all(body.as_bytes())
        .await
        .map_err(|error| error.to_string())?;
    let mut read = Vec::new();
    stream
        .read_to_end(&mut read)
        .await
        .map_err(|error| error.to_string())?;
    let split = read
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or("no header")?;
    let answer: Value =
        serde_json::from_slice(&read[split + 4..]).map_err(|error| error.to_string())?;
    let session = answer["session"]
        .as_str()
        .ok_or_else(|| answer.to_string())?
        .to_string();
    let stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|error| error.to_string())?;
    let request = format!("ws://127.0.0.1:{port}/channel")
        .into_client_request()
        .map_err(|error| error.to_string())?;
    let (mut channel, _) = tokio_tungstenite::client_async(request, stream)
        .await
        .map_err(|error| error.to_string())?;
    channel
        .send(Message::text(json!({"session": session}).to_string()))
        .await
        .map_err(|error| error.to_string())?;
    match next(&mut channel).await {
        Some(ready) if ready["ready"] == true => Ok(channel),
        other => Err(format!("{other:?}")),
    }
}

/// 调一个方法，等它的回应（推来的跳过：开始看以前没有推的）。断了的是空的。
async fn ask(channel: &mut Channel, id: u64, method: &str, params: Value) -> Option<Value> {
    let request = json!({"id": id, "method": method, "params": params}).to_string();
    channel.send(Message::text(request)).await.ok()?;
    loop {
        let frame = next(channel).await?;
        if frame["id"] == id {
            return Some(frame["result"].clone());
        }
    }
}

/// 下一帧 JSON；断了的是空的。
async fn next(channel: &mut Channel) -> Option<Value> {
    loop {
        match channel.next().await? {
            Ok(Message::Text(text)) => return serde_json::from_str(&text).ok(),
            Ok(Message::Close(_)) | Err(_) => return None,
            Ok(_) => {}
        }
    }
}

/// 本机此刻的时区偏移，分钟。
fn offset_minutes() -> i32 {
    jiff::Zoned::now().offset().seconds() / 60
}

/// 印一行。
fn say(out: &mut dyn Write, line: &str) {
    if writeln!(out, "{line}").is_err() {}
}

/// 印一段不换行，马上冲出去。
fn write(out: &mut dyn Write, text: &str) {
    if write!(out, "{text}").and_then(|()| out.flush()).is_err() {}
}

#[cfg(test)]
mod tests;
