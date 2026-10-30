//! 给别的包调的方法（施工 O-42，`onebot.md` 第一条「给别的包调的方法」，`extending.md`「调接入通讯平台 的方法」）：清单 `[export]`
//! 里的两个，别的包经核心的 `package.call` 调，核心反向发 `method.call`（带 `from`：调的包的编号）。读的一头把这两个照推送交到
//! 这里：会话到场所的表、机器人号的连接都在这一头。
//!
//! - `chat.action`：替别的包调一个 OneBot 动作，NapCat 回什么交回什么（拿掉桥自己编的 `echo`）。不挑动作：调得到的包是人批过
//!   `packages.call` 的（施工单「要拍板的」第 1 条）。另起任务等 NapCat（`chores`，同平台工具），跟核心的那一头不等。
//! - `chat.venue`：一个会话是哪个场所、收进它的机器人号；支线照它的群（同平台工具，施工 O-36）。认不得的场所是 `null`。施工
//!   S-1 上多 `origin`：那条线这一轮叫她做的那一条（同平台工具的 `Call::origin`）：谁发的、引用了哪条谁的、@ 了谁，每个人带号
//!   和名字（群里照这个群的成员缓存、他说过的话；私聊的引用不知道是谁）；没有的是 `null`。别的包（赞助记录）照它认「记谁」。
//!   施工 P-1 中多 `plugins`：这个场所开着的插件的编号（同告诉核心的 `venue.plugins`）：工具以外的东西（表情包包每轮的提醒）照它
//!   决定在这个场所出不出声。
//! - `chat.media`（施工 I-39，`media`）：那个会话所在场所里一条消息的第几样图和它的来历（表情包包偷图）。

use std::sync::Arc;
use std::time::Instant;

use miyu_client::ids::ExternalId;
use serde_json::{Map, Value, json};

use super::Route;
use crate::TARGET;
use crate::chat::CallError;
use crate::chat::{Bots, Id, Platform, To, Unclaimed};
use crate::core::Answerer;
use crate::core::methods::{MEDIA, VENUE};

mod media;

/// 动作名最长几个字符。
const NAME_MAX: usize = 64;

/// 读好的一次 `chat.action`。
#[derive(Debug, Clone, PartialEq)]
struct Request {
    /// OneBot 动作名。
    action: String,
    /// 动作的参数：对象，没写的是空的。
    params: Map<String, Value>,
    /// 经哪个机器人号：没写的照这个平台连着的号里最小的那一个（施工 I-70 上：不跨平台）。
    bot: Option<Id>,
}

impl Request {
    /// 读 `chat.action` 的参数：`action` 是小写字母、数字、`_`、`.`，1 到 [`NAME_MAX`] 个（NapCat 的扩展动作有 `.` `_` 开头的）；
    /// `params` 不写或是对象；`bot` 不写或是整数。别的格不认，多写的不管。写错的交回空的。
    fn read(args: &Value) -> Option<Request> {
        let action = args["action"].as_str()?;
        let named = (1..=NAME_MAX).contains(&action.chars().count())
            && action
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.');
        if !named {
            return None;
        }
        let params = match &args["params"] {
            Value::Null => Map::new(),
            Value::Object(params) => params.clone(),
            _ => return None,
        };
        let bot = match &args["bot"] {
            Value::Null => None,
            bot => Some(serde_json::from_value::<Id>(bot.clone()).ok()?),
        };
        Some(Request {
            action: action.to_string(),
            params,
            bot,
        })
    }
}

impl Route {
    /// 核心转来的一次 `method.call`（`pushed`），方法是 `chat.action`、[`VENUE`] 或 [`MEDIA`]：写错的当场拒（`bad_params`），`chat.venue`
    /// 当场答，`chat.action`、`chat.media` 另起任务等 NapCat 再答。写了别的平台的拒（施工 I-1）；`chat.action` 只给号，平台照
    /// `Platforms::ours` 认，挂了几个平台又没写的也拒（施工 I-70 上）；另外两个照会话认，平台在会话的场所里。
    pub(super) fn exported(&mut self, pushed: &Value) {
        let id = pushed["id"].clone();
        let params = &pushed["params"];
        let from = params["from"].as_str().unwrap_or_default().to_string();
        let args = &params["params"];
        let answerer = self.core.answerer();
        // 写了别的平台的不是这个驱动的（施工 I-1）。
        let platform = self.platforms.ours(args);
        if platform == Err(Unclaimed::Other) {
            tracing::info!(target: TARGET, from, "export refused, another platform");
            answerer.refuse(&id, "bad_params");
            return;
        }
        if params["method"] == VENUE {
            match args["session"].as_str() {
                Some(session) => answerer.answer(&id, self.venue_of(session)),
                None => answerer.refuse(&id, "bad_params"),
            }
            return;
        }
        if params["method"] == MEDIA {
            self.media_exported(id, from, args, answerer);
            return;
        }
        // 只给号的：挂了几个平台又没写的不知道是哪个平台的号（施工 I-70 上）。
        let platform = match platform {
            Ok(platform) => platform,
            Err(why) => {
                tracing::info!(target: TARGET, from, why = why.name(), "export action refused");
                answerer.refuse(&id, "bad_params");
                return;
            }
        };
        let Some(request) = Request::read(args) else {
            tracing::info!(target: TARGET, from, "export action refused");
            answerer.refuse(&id, "bad_params");
            return;
        };
        let bots = self.platforms.bots(platform);
        self.chores.spawn(act(id, from, request, bots, answerer));
    }

    /// 会话 `session` 是哪个场所、收进它的机器人号、那条线这一轮叫她做的那一条：支线照它的群；认不得的场所是 `null`、没有
    /// `bot`。
    fn venue_of(&self, session: &str) -> Value {
        let main = self.main_of(session);
        match self.peers.get(&main) {
            Some(peer) => {
                let origin = self.origin_of(session, &main, (peer.platform, peer.to.clone()));
                let plugins =
                    self.hooks
                        .plugins_on(peer.platform, Some(&peer.bot), peer.to.clone());
                json!({"platform": peer.platform.name(), "venue": peer.venue.as_str(), "bot": peer.bot, "origin": origin, "plugins": plugins})
            }
            None => json!({"venue": null}),
        }
    }

    /// 线 `line`（主线 `main`，平台 `platform` 上的对方 `to`）这一轮叫她做的那一条（模块的说明）；没有的是 `null`。
    fn origin_of(&self, line: &str, main: &str, (platform, to): (Platform, To)) -> Value {
        match to {
            To::Group(group) => {
                let Some(projection) = self.groups.get(main) else {
                    return Value::Null;
                };
                let Some(origin) = projection.origin(line) else {
                    return Value::Null;
                };
                let cached = self.members.names_in(platform, &group, Instant::now());
                let person = |who: &ExternalId| {
                    let name = number(who)
                        .and_then(|user| cached.get(&user).cloned())
                        .or_else(|| projection.name_of(who));
                    json!({"id": who.as_str(), "name": name})
                };
                let quote = origin.quote.map(
                    |quote| json!({"msg": quote.msg, "sender": quote.sender.as_ref().map(&person)}),
                );
                let mentions: Vec<Value> = origin.mentions.iter().map(&person).collect();
                json!({"sender": person(&origin.sender), "quote": quote, "mentions": mentions})
            }
            To::Private(user) => {
                let Ok(other) = platform.person(&user) else {
                    return Value::Null;
                };
                let quote = self
                    .quotes
                    .get(main)
                    .cloned()
                    .flatten()
                    .map(|msg| json!({"msg": msg, "sender": null}));
                json!({"sender": {"id": other.as_str(), "name": null}, "quote": quote, "mentions": []})
            }
        }
    }
}

/// 平台上的人 `id` 的号。
fn number(id: &ExternalId) -> Option<Id> {
    Id::new(id.as_str().rsplit_once(':')?.1)
}

/// 办一次 `chat.action`：经那个号这时的连接调动作，NapCat 回了的（成没成）原样交回、拿掉 `echo`；没连着的、断了的拒
/// `not_connected`，等不到的拒 `unanswered`。运行日志不记参数、不记回答。
async fn act(id: Value, from: String, request: Request, bots: Arc<Bots>, answerer: Answerer) {
    let link = match request.bot {
        Some(bot) => bots.get(&bot),
        None => bots.first().map(|(_, link)| link),
    };
    let Some(link) = link else {
        tracing::info!(target: TARGET, from, action = request.action, why = "not_connected", "export action failed");
        answerer.refuse(&id, "not_connected");
        return;
    };
    let called = link
        .raw(&request.action, Value::Object(request.params))
        .await;
    let mut reply = match called {
        Ok(reply) => reply,
        Err(CallError::Timeout) => {
            tracing::info!(target: TARGET, from, action = request.action, why = "unanswered", "export action failed");
            answerer.refuse(&id, "unanswered");
            return;
        }
        // 这个平台没有原样的调用（施工 I-82）。
        Err(CallError::Unsupported) => {
            tracing::info!(target: TARGET, from, action = request.action, why = "unsupported", "export action failed");
            answerer.refuse(&id, "unsupported");
            return;
        }
        // 原样的调用不会回 `Failed`（`Link::raw`）：照断了答。
        Err(CallError::Closed | CallError::Failed { .. }) => {
            tracing::info!(target: TARGET, from, action = request.action, why = "not_connected", "export action failed");
            answerer.refuse(&id, "not_connected");
            return;
        }
    };
    if let Some(reply) = reply.as_object_mut() {
        reply.remove("echo");
    }
    tracing::info!(target: TARGET, from, action = request.action, retcode = %reply["retcode"], "export action");
    answerer.answer(&id, reply);
}

#[cfg(test)]
mod tests;
