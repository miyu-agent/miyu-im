//! 一条 NapCat 的连接（`onebot.md` 第一条「怎么走」第 2 到 6 条）：一帧一条 JSON，回应交给在等的调用，消息（私聊、群）、
//! 撤回（施工 O-22）和禁言、解禁（施工 O-25 中）交给跟核心的那一头，别的事件记一行调试日志就丢。号认出来了（`X-Self-ID`、
//! 第一条事件）也告诉跟核心的那一头（`Event::Connected`，施工 O-25 中：排着的照先后发），和消息走同一条队，先后一致。连上就
//! 调一次 `get_version_info`，把实现的名字和版本记进运行日志（第 3 条），也记在这条连接上，后台页的 `status`、状态文件照它说（施工
//! O-16）。号认出来、断开、问到实现，都叫一声状态文件（施工 O-18）。
//!
//! 往 NapCat 写的都经一个写的任务（回话、`get_version_info`、被顶掉时的关闭帧）。断开时：在等的调用都算失败，号还是这一条
//! 的拿掉，说一行；桥不退，等 NapCat 自己重连（第 11 条）。

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::Role;

use crate::onebot::{self, Event, Frame, Wire};
use miyu_im::TARGET;
use miyu_im::chat::{Implementation, Link};
use miyu_im::driver::Gate;
use miyu_im::serve::Notice;

/// 交给群管理照先后同步的那一头（施工 Q-3）。
type Syncing = mpsc::UnboundedSender<crate::admin::sync::Moderated>;

use crate::admin::spam::Spamming;
use crate::backfill::{self, Seen};

/// 跑一条升级好的连接，直到断开。`bot` 是 `X-Self-ID` 报的号。
pub(super) async fn run<S>(io: S, gate: &Arc<Gate>, mut bot: Option<i64>)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let ws = WebSocketStream::from_raw_socket(io, Role::Server, None).await;
    let (mut sink, mut frames) = ws.split();
    let (out, mut outgoing) = mpsc::channel::<Message>(gate.tuning().write_queue);
    // 写的任务：发的一头都放下了（连接断了、被顶掉了），或者写了关闭帧，就停。
    tokio::spawn(async move {
        while let Some(message) = outgoing.recv().await {
            let closing = matches!(message, Message::Close(_));
            if sink.send(message).await.is_err() || closing {
                break;
            }
        }
        if let Err(error) = sink.close().await {
            tracing::debug!(target: TARGET, error = %error, "websocket not closed cleanly");
        }
    });
    let wire = Arc::new(Wire::new(out, gate.tuning().call_timeout()));
    let link: Arc<dyn Link> = wire.clone();
    // 群管理同步别的管理员做的（施工 Q-3）：这条连接一个，照先后办。
    let syncing = crate::admin::sync::start(gate, &link);
    // 刷屏处理（施工 Q-5）：这条连接一份，照读到的先后认。
    let mut spamming = Spamming::new(gate, &link);
    // 补拉消息（施工 I-15）：号认出来以后才有。
    let mut seen = None;
    // 跟核心的那一头不收了（桥在停）：这条连接不再读。
    let mut reading = match bot {
        Some(bot) => known(gate, bot, &link, &mut seen).await,
        None => true,
    };
    tracing::info!(target: TARGET, bot, "napcat connected");
    gate.tell(Notice::Connected { bot });
    let probe = version(&wire, gate);
    tokio::pin!(probe);
    let mut probed = false;
    while reading {
        tokio::select! {
            frame = frames.next() => match frame {
                Some(Ok(Message::Text(text))) => {
                    reading = frame_in(gate, &text, (&mut bot, &mut seen), (&wire, &link, &syncing, &mut spamming)).await;
                }
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => reading = false,
                Some(Ok(_)) => {}
            },
            () = &mut probe, if !probed => probed = true,
        }
    }
    wire.calls.close();
    if let Some(seen) = seen.as_mut() {
        seen.flush();
    }
    if let Some(bot) = bot {
        gate.bots().remove(bot, &link);
        gate.changed();
    }
    tracing::info!(target: TARGET, bot, "napcat disconnected");
    gate.tell(Notice::Disconnected { bot });
}

/// 一帧文字。跟核心的那一头不收了（桥在停）交回假。
async fn frame_in(
    gate: &Arc<Gate>,
    text: &str,
    (bot, seen): (&mut Option<i64>, &mut Option<Seen>),
    (wire, link, syncing, spamming): (&Wire, &Arc<dyn Link>, &Syncing, &mut Spamming),
) -> bool {
    let frame: Value = match serde_json::from_str(text) {
        Ok(frame) => frame,
        Err(error) => {
            tracing::debug!(target: TARGET, error = %error, "frame not understood");
            return true;
        }
    };
    if bot.is_none()
        && let Some(id) = onebot::self_id(&frame)
    {
        *bot = Some(id);
        if !known(gate, id, link, seen).await {
            return false;
        }
    }
    // 通知、请求原样交给挂在 `chat.event` 的包（施工 O-43 上）：桥认不认它、怎么办都不影响。
    if matches!(frame["post_type"].as_str(), Some("notice" | "request"))
        && let Some(id) = onebot::self_id(&frame)
    {
        gate.event(id, onebot::event_to(&frame), &frame);
        // 群里别的管理员禁言、解禁、踢人（施工 Q-3，群管理）：另起任务办，要问 NapCat 的名字、身份，回应在这条连接上读。
        crate::admin::sync::seen(syncing, &frame);
        // 加群申请（施工 Q-7，入群审批）：另起任务审。
        crate::admin::approval::seen(gate, link, &frame);
    }
    match onebot::read(frame) {
        Frame::Reply(reply) => {
            if !wire.calls.answer(reply) {
                tracing::debug!(target: TARGET, "reply nobody waits for");
            }
        }
        Frame::Event(event) => {
            noted(&event);
            // 补拉消息（施工 I-15）：记下每个群收到的最后一条（刷屏的也算，平台上有它）。
            if let (Event::Group { group, posted }, Some(seen)) = (&event, seen.as_mut()) {
                seen.note(*group, posted.time);
            }
            // 刷屏的（施工 Q-5，群管理）：第三条起不交给 im。
            if let Event::Group { group, posted } = &event
                && !spamming.seen(*group, posted)
            {
                return true;
            }
            return gate.inbound(event).await;
        }
        Frame::Other(kind) => tracing::debug!(target: TARGET, kind, "event ignored"),
    }
    true
}

/// 读出来的一件事记一行调试日志：谁、哪一条、几个字，不记原文、请求的标记。
fn noted(event: &Event) {
    match event {
        Event::Private(posted) => tracing::debug!(
            target: TARGET,
            user = posted.user,
            message = posted.message_id,
            chars = posted.text.chars().count(),
            "private message"
        ),
        Event::Group { group, posted } => tracing::debug!(
            target: TARGET,
            group,
            user = posted.user,
            message = posted.message_id,
            chars = posted.text.chars().count(),
            "group message"
        ),
        Event::Backfilled { group, posted } => tracing::debug!(
            target: TARGET,
            group,
            user = posted.user,
            message = posted.message_id,
            "backfilled message"
        ),
        Event::Recalled(recall) => tracing::debug!(
            target: TARGET,
            group = recall.group,
            user = recall.user,
            message = recall.message_id,
            "recall"
        ),
        Event::Muted { group, seconds, .. } => {
            tracing::debug!(target: TARGET, group, seconds, "muted");
        }
        Event::Unmuted { group, .. } => tracing::debug!(target: TARGET, group, "unmuted"),
        Event::Connected { bot } => tracing::debug!(target: TARGET, bot, "bot known"),
        Event::Befriend { user, .. } => tracing::debug!(target: TARGET, user, "friend request"),
        Event::Noted { kind, .. } => tracing::debug!(target: TARGET, kind, "built-in note"),
        Event::Asked { .. } => tracing::debug!(target: TARGET, "built-in model call"),
        Event::Invited { group, user, .. } => {
            tracing::debug!(target: TARGET, group, user, "group invite");
        }
        Event::Happened(happened) => tracing::debug!(
            target: TARGET,
            group = happened.group,
            user = happened.user,
            change = happened.change.as_str(),
            "group change"
        ),
    }
}

/// 号 `bot` 认出来了：登记这条连接（[`register`]），读它记着的、照记着的补拉（施工 I-15）。跟核心的那一头不收了交回假。
async fn known(gate: &Arc<Gate>, bot: i64, link: &Arc<dyn Link>, seen: &mut Option<Seen>) -> bool {
    if !register(gate, bot, link).await {
        return false;
    }
    let loaded = Seen::load(gate.store(), bot);
    backfill::start(gate, link, bot, loaded.groups());
    *seen = Some(loaded);
    true
}

/// 号 `bot` 现在用 `link`：顶掉的那一条在等的调用都算失败，给它发一帧关闭；告诉跟核心的那一头这个号连上了（施工 O-25 中，
/// 「施工时定的」第 126 条）。跟核心的那一头不收了（桥在停）交回假。
async fn register(gate: &Gate, bot: i64, link: &Arc<dyn Link>) -> bool {
    if let Some(old) = gate.bots().insert(bot, Arc::clone(link)) {
        old.close();
        tracing::info!(target: TARGET, bot, "napcat connection replaced");
    }
    gate.changed();
    gate.inbound(Event::Connected { bot }).await
}

/// 问对端是谁（第 3 条）：实现的名字、版本、协议版本记进运行日志；名字和版本记进这条连接，叫一声 `changed`。
async fn version(wire: &Wire, gate: &Gate) {
    match wire
        .calls
        .call(&wire.out, "get_version_info", json!({}))
        .await
    {
        Ok(reply) => {
            let data = &reply["data"];
            let known = Implementation {
                name: data["app_name"].as_str().unwrap_or_default().to_string(),
                version: data["app_version"].as_str().unwrap_or_default().to_string(),
            };
            if wire.implementation.set(known).is_err() {
                // 一条连接只问一次：不会已经有了。
            }
            gate.changed();
            tracing::info!(
                target: TARGET,
                app_name = data["app_name"].as_str().unwrap_or_default(),
                app_version = data["app_version"].as_str().unwrap_or_default(),
                protocol_version = data["protocol_version"].as_str().unwrap_or_default(),
                "napcat version"
            );
        }
        Err(error) => tracing::warn!(target: TARGET, error = ?error, "napcat version unknown"),
    }
}
