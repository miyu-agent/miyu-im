//! 一个连接跑起来（施工 I-183 中上，设计 07 第 3.5 节）：
//!
//! 1. 没有令牌的不连，报「没有令牌」。代理地址读不懂的不连，报「代理不对」。
//! 2. `getMe`：认出机器人的号（照回的 `id`，不照令牌冒号前那串）、名字（`first_name` 加 `last_name`）、用户名、隐私模式（
//!    `can_read_all_group_messages` 为假是开着）。401 报「令牌失效」，不再试（换了令牌配置变了才重来）；连不上的退避着再试。
//! 3. 认出来了把连接登记进宿主的 `Bots`，告诉宿主这个号连上了（排着的照先后发）。
//! 4. 长轮询 `getUpdates`：Telegram 挂着不回，有消息才回，最多挂 [`POLL`]；收到第几条记进状态目录（`<号>.offset`），重启接着收，不重收。
//!    收到的消息认成宿主的事交过去（[`crate::receive`]，I-183 中下）。
//! 5. 没成的：409（别处也在收）报「别处也在收消息」、拿掉登记，过 [`ELSEWHERE`] 再试，不抢；429 照它说的等；401 同第 2 条；连不上、
//!    等不到的（挂着超过 [`POLL`] 加 [`GRACE`] 还没回，当断了，AstrBot #8314 的「跑一阵就收不到了」）报「连不上」、拿掉登记，退避着
//!    再试，好了重新登记。
//!
//! 停（配置换了、桥在停）就是这个任务被放下：放下时拿掉登记（[`Registered`] 的 `Drop`）。

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};

use miyu_im::TARGET;
use miyu_im::chat::{Event, Id, Link};
use miyu_im::driver::Gate;

use crate::api::{Api, Failure};
use crate::known::Known;
use crate::link::Bot;
use crate::receive::{self, Me};
use crate::send::{self, Queue};
use crate::settings::Connection;

/// 长轮询让 Telegram 最多挂多久（`getUpdates` 的 `timeout`）。
pub const POLL: Duration = Duration::from_secs(50);

/// 挂着的请求比 [`POLL`] 多等这么久还没回，当断了。
pub const GRACE: Duration = Duration::from_secs(20);

/// 别的调用最多等多久。
const CALL: Duration = Duration::from_secs(30);

/// 409（别处也在收）以后隔多久再试。
pub const ELSEWHERE: Duration = Duration::from_secs(30);

/// 连不上时退避的头一次、最长。
const BACKOFF: (Duration, Duration) = (Duration::from_secs(1), Duration::from_secs(60));

/// 一个连接报给页面的样子（[`State::fields`]）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// 连接表里第几个。
    pub index: usize,
    /// 写了令牌。
    pub token: bool,
    /// 代理（存的写法）。
    pub proxy: String,
    /// 认出来的机器人：号、用户名、名字。
    pub bot: Option<(Id, String, String)>,
    /// 隐私模式开着；还没问到的是空的。
    pub privacy: Option<bool>,
    /// 连着（登记在宿主的 `Bots` 里）。
    pub connected: bool,
    /// 毛病：`no_token`、`proxy`、`token`（401）、`elsewhere`（409）、`network`；没有的是空的。
    pub trouble: Option<&'static str>,
}

impl State {
    /// 报给宿主的一格（`status` 的 `connections` 里一项）。
    pub fn fields(&self) -> Value {
        let mut fields = json!({
            "index": self.index,
            "token": if self.token { "set" } else { "none" },
            "proxy": self.proxy,
            "connected": self.connected,
        });
        if let Some((bot, username, name)) = &self.bot {
            fields["bot"] = json!(bot.as_str());
            fields["username"] = json!(username);
            fields["name"] = json!(name);
        }
        if let Some(privacy) = self.privacy {
            fields["privacy"] = json!(privacy);
        }
        if let Some(trouble) = self.trouble {
            fields["trouble"] = json!(trouble);
        }
        fields
    }
}

/// 这个连接的状态和改了以后叫谁（[`crate::running`] 照它报给宿主）。
pub struct Shared {
    /// 状态。
    pub state: Mutex<State>,
    /// 改了以后叫一声。
    pub changed: Box<dyn Fn() + Send + Sync>,
}

impl Shared {
    /// 改状态，叫一声。
    fn update(&self, change: impl FnOnce(&mut State)) {
        change(&mut self.state.lock().unwrap_or_else(PoisonError::into_inner));
        (self.changed)();
    }
}

/// 登记在宿主 `Bots` 里的一个号：放下时拿掉（同一个号已经换了别的连接的不动）。
struct Registered {
    gate: Arc<Gate>,
    bot: Id,
    link: Arc<dyn Link>,
}

impl Drop for Registered {
    fn drop(&mut self) {
        self.link.close();
        self.gate.bots().remove(&self.bot, &self.link);
        self.gate.changed();
    }
}

/// 跑连接 `connection`（Bot API 的地址 `api`）：放下时停。
pub async fn run(gate: Arc<Gate>, connection: Connection, api: String, shared: Arc<Shared>) {
    let Some(token) = connection.token else {
        shared.update(|state| state.trouble = Some("no_token"));
        return;
    };
    let client = match Api::new(&api, &token, &connection.proxy) {
        Ok(client) => client,
        Err(reason) => {
            tracing::warn!(target: TARGET, index = connection.index, reason = %reason, "telegram connection not started");
            shared.update(|state| state.trouble = Some("proxy"));
            return;
        }
    };
    let Some((bot, name, username)) = whoami(&client, &shared).await else {
        return;
    };
    // 收的时候记下话题、名字、群名，发的时候照它们（施工 I-183 中下）；发的那条队整个连接一条，重连不换。
    let known = Arc::new(Known::default());
    let (queue, writer) = send::queue(client.clone(), Arc::clone(&known));
    tokio::spawn(writer);
    let me = Me {
        bot: &bot,
        username: &username,
    };
    let offset_file = gate.store().join(format!("{bot}.offset"));
    let mut offset = read_offset(&offset_file).await;
    let mut registered = None;
    let mut backoff = BACKOFF.0;
    loop {
        if registered.is_none() {
            registered = register(&gate, (&bot, &name), &shared, (&known, &queue)).await;
            if registered.is_none() {
                return;
            }
        }
        let params =
            json!({"offset": offset, "timeout": POLL.as_secs(), "allowed_updates": ["message"]});
        match client.call("getUpdates", &params, POLL + GRACE).await {
            Ok(updates) => {
                backoff = BACKOFF.0;
                shared.update(|state| state.trouble = None);
                // 先交给宿主，再记收到了第几条：交到一半停了的，重启再收一遍（宿主照编号去重）。
                for update in updates.as_array().into_iter().flatten() {
                    if let Some(event) = receive::event(&me, &update["message"], &known)
                        && !gate.inbound(event).await
                    {
                        return;
                    }
                }
                let next = updates
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|update| update["update_id"].as_i64())
                    .max();
                if let Some(last) = next
                    && last + 1 > offset
                {
                    offset = last + 1;
                    write_offset(&offset_file, offset).await;
                }
            }
            Err(Failure::Retry(wait)) => tokio::time::sleep(wait).await,
            Err(failure) => {
                let (trouble, wait) = match &failure {
                    Failure::Unauthorized => ("token", None),
                    Failure::Conflict => ("elsewhere", Some(ELSEWHERE)),
                    _ => ("network", Some(backoff)),
                };
                tracing::warn!(target: TARGET, bot = %bot, failure = ?failure, "telegram updates failed");
                registered = None;
                shared.update(|state| {
                    state.trouble = Some(trouble);
                    state.connected = false;
                });
                let Some(wait) = wait else {
                    return;
                };
                tokio::time::sleep(wait).await;
                if trouble == "network" {
                    backoff = (backoff * 2).min(BACKOFF.1);
                }
            }
        }
    }
}

/// `getMe`：认出号、名字、用户名，报进状态。401 的报「令牌失效」交回空的；连不上的退避着一直试。
async fn whoami(client: &Api, shared: &Shared) -> Option<(Id, String, String)> {
    let mut backoff = BACKOFF.0;
    loop {
        match client.call("getMe", &json!({}), CALL).await {
            Ok(me) => {
                let Some(bot) = me["id"].as_i64().and_then(|id| Id::new(&id.to_string())) else {
                    tracing::warn!(target: TARGET, "telegram getMe without id");
                    shared.update(|state| state.trouble = Some("network"));
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(BACKOFF.1);
                    continue;
                };
                let name = full_name(&me);
                let username = me["username"].as_str().unwrap_or_default().to_string();
                let privacy = me["can_read_all_group_messages"] != true;
                tracing::info!(target: TARGET, bot = %bot, privacy, "telegram bot known");
                shared.update(|state| {
                    state.bot = Some((bot.clone(), username.clone(), name.clone()));
                    state.privacy = Some(privacy);
                    state.trouble = None;
                });
                return Some((bot, name, username));
            }
            Err(Failure::Unauthorized) => {
                tracing::warn!(target: TARGET, "telegram token refused");
                shared.update(|state| state.trouble = Some("token"));
                return None;
            }
            Err(Failure::Retry(wait)) => tokio::time::sleep(wait).await,
            Err(failure) => {
                tracing::warn!(target: TARGET, failure = ?failure, "telegram getMe failed");
                shared.update(|state| state.trouble = Some("network"));
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(BACKOFF.1);
            }
        }
    }
}

/// 名字：`first_name` 加 `last_name`（设计 07：AstrBot 只取用户名，没用户名的人都叫「Unknown」）。
pub fn full_name(user: &Value) -> String {
    let first = user["first_name"].as_str().unwrap_or_default().trim();
    let last = user["last_name"].as_str().unwrap_or_default().trim();
    match (first.is_empty(), last.is_empty()) {
        (false, false) => format!("{first} {last}"),
        (false, true) => first.to_string(),
        _ => last.to_string(),
    }
}

/// 把号 `bot` 登记进宿主的 `Bots`，告诉宿主连上了。宿主不收了（桥在停）交回空的。
async fn register(
    gate: &Arc<Gate>,
    (bot, name): (&Id, &str),
    shared: &Shared,
    (known, queue): (&Arc<Known>, &Queue),
) -> Option<Registered> {
    let link: Arc<dyn Link> =
        Arc::new(Bot::new(name.to_string(), Arc::clone(known), queue.clone()));
    if let Some(old) = gate.bots().insert(bot.clone(), Arc::clone(&link)) {
        old.close();
    }
    let registered = Registered {
        gate: Arc::clone(gate),
        bot: bot.clone(),
        link,
    };
    shared.update(|state| state.connected = true);
    tracing::info!(target: TARGET, bot = %bot, "telegram connected");
    gate.inbound(Event::Connected { bot: bot.clone() })
        .await
        .then_some(registered)
}

/// 读记下的下一条的编号；没有、读不出的是 0（从 Telegram 留着的最早一条收）。
async fn read_offset(file: &PathBuf) -> i64 {
    tokio::fs::read_to_string(file)
        .await
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0)
}

/// 记下一条的编号：先写旁边的临时文件再改名，写到一半断电不留半个数。写不进的记一行日志，照跑（重启会重收几条，宿主照编号去重）。
async fn write_offset(file: &PathBuf, offset: i64) {
    let temporary = file.with_extension("offset.tmp");
    let written = async {
        if let Some(dir) = file.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }
        tokio::fs::write(&temporary, offset.to_string()).await?;
        tokio::fs::rename(&temporary, file).await
    };
    if let Err(error) = written.await {
        tracing::warn!(target: TARGET, error = %error, "telegram offset not saved");
    }
}

#[cfg(test)]
mod tests;
