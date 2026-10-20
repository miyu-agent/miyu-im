//! OneBot 的一条连接（施工 I-2 上）：宿主要做的事（[`Link`]）照 OneBot v11 拼成动作，放进这条反向 WebSocket 的写队列，回应照
//! `echo` 配对（[`Calls`]），读成宿主认的样子（`replies`）。

use std::sync::OnceLock;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use super::media::{get_media, get_msg};
use super::replies::{self, FRIEND_LIST, GROUP_INFO, GROUP_LIST, LOGIN_INFO, MEMBER_LIST};
use super::{
    Calls, MEMBER_INFO, delete_msg, emoji_like, forward_messages, forward_to, friend_add,
    get_forward_msg, group_ban, group_kick, image_to, member_info, message_to, node, number,
    picture_to, poke, upload_to,
};
use miyu_im::chat::{
    self, CallError, Fetch, Fetched, Friend, Group, Implementation, Link, Located, Member, Msg,
    Node, Outgoing, Reply, To,
};

/// 一条 NapCat 的连接：往它写的队列，它上面在等回应的调用，对端是哪个实现。
#[derive(Debug)]
pub struct Wire {
    /// 往这条连接写的队列。
    pub out: mpsc::Sender<Message>,
    /// 这条连接上在等回应的调用。
    pub calls: Calls,
    /// 对端是哪个实现：连上以后问 `get_version_info`，回了才有（`onebot.md` 第一条「怎么走」第 3 条）。
    pub implementation: OnceLock<Implementation>,
}

impl Wire {
    /// 一条新的连接：往 `out` 写，一个回应最多等 `timeout`。
    pub fn new(out: mpsc::Sender<Message>, timeout: Duration) -> Wire {
        Wire {
            out,
            calls: Calls::new(timeout),
            implementation: OnceLock::new(),
        }
    }

    /// 调一个动作，等回应。
    async fn call(&self, (action, params): (&str, Value)) -> Result<Value, CallError> {
        self.calls.call(&self.out, action, params).await
    }

    /// 调一个动作，成了就算，回应不看。
    fn done<'a>(&'a self, call: (&'a str, Value)) -> Reply<'a, ()> {
        Box::pin(async move { self.call(call).await.map(drop) })
    }

    /// 调一个动作，回应的 `data` 照 `read` 读。
    fn read<'a, T: 'a>(
        &'a self,
        call: (&'a str, Value),
        read: impl FnOnce(&Value) -> T + Send + 'a,
    ) -> Reply<'a, T> {
        Box::pin(async move { Ok(read(&self.call(call).await?["data"])) })
    }
}

impl Link for Wire {
    fn begin<'a>(&'a self, to: To, outgoing: Outgoing<'a>) -> Reply<'a, chat::Pending> {
        let (action, params) = match outgoing {
            Outgoing::Text { text, lead } => message_to(to, text, lead),
            Outgoing::Image { file, sticker } => image_to(to, file, sticker),
            Outgoing::Picture { file, lead } => picture_to(to, file, lead),
            Outgoing::Forward {
                bot,
                nickname,
                nodes,
            } => forward_to(to, bot, nickname, nodes),
            Outgoing::Upload { file, name } => upload_to(to, file, name),
        };
        Box::pin(async move {
            let pending = self.calls.begin(&self.out, action, params).await?;
            let sent: chat::Pending = Box::pin(async move {
                let reply = pending.wait().await?;
                Ok(number(&reply["data"]["message_id"]))
            });
            Ok(sent)
        })
    }

    fn recall<'a>(&'a self, message: Msg<'a>) -> Reply<'a, ()> {
        self.done(match message {
            Msg::Number(id) => delete_msg(id),
            Msg::Text(id) => delete_msg(id),
        })
    }

    fn mute(&self, group: i64, user: i64, seconds: u64) -> Reply<'_, ()> {
        self.done(group_ban(group, user, seconds))
    }

    fn kick(&self, group: i64, user: i64, block: bool) -> Reply<'_, ()> {
        self.done(group_kick(group, user, block))
    }

    fn poke(&self, to: To, user: i64) -> Reply<'_, ()> {
        self.done(poke(to, user))
    }

    fn react<'a>(&'a self, message: &'a str, emoji: &'a str, on: bool) -> Reply<'a, ()> {
        self.done(emoji_like(message, emoji, on))
    }

    fn befriend<'a>(&'a self, flag: &'a str) -> Reply<'a, ()> {
        self.done(friend_add(flag))
    }

    fn message<'a>(&'a self, id: &'a str) -> Reply<'a, Fetched> {
        self.read(get_msg(id), replies::fetched)
    }

    fn fetch<'a>(&'a self, fetch: Fetch, id: &'a str, wait: Duration) -> Reply<'a, Located> {
        Box::pin(async move {
            let (action, params) = get_media(fetch, id);
            let pending = self.calls.begin(&self.out, action, params).await?;
            let reply = pending.wait_for(wait).await?;
            Ok(replies::located(&reply["data"]))
        })
    }

    fn forwarded<'a>(&'a self, id: &'a str) -> Reply<'a, Option<Vec<Node>>> {
        self.read(get_forward_msg(id), |data| {
            forward_messages(data).map(|messages| messages.iter().map(node).collect())
        })
    }

    fn members(&self, group: i64) -> Reply<'_, Vec<Member>> {
        self.read((MEMBER_LIST, json!({"group_id": group})), replies::members)
    }

    fn member(&self, group: i64, user: i64) -> Reply<'_, Member> {
        self.read((MEMBER_INFO, member_info(group, user)), move |data| {
            let mut member = replies::member(data).unwrap_or(Member {
                user,
                name: None,
                rank: None,
            });
            member.user = user;
            member
        })
    }

    fn groups(&self) -> Reply<'_, Vec<Group>> {
        self.read((GROUP_LIST, json!({})), replies::groups)
    }

    fn friends(&self) -> Reply<'_, Vec<Friend>> {
        self.read((FRIEND_LIST, json!({})), replies::friends)
    }

    fn group_name(&self, group: i64) -> Reply<'_, Option<String>> {
        self.read(
            (GROUP_INFO, json!({"group_id": group})),
            replies::group_name,
        )
    }

    fn nickname(&self) -> Reply<'_, Option<String>> {
        self.read((LOGIN_INFO, json!({})), replies::nickname)
    }

    fn raw<'a>(&'a self, action: &'a str, params: Value) -> Reply<'a, Value> {
        Box::pin(async move {
            let pending = self.calls.begin(&self.out, action, params).await?;
            pending.reply().await
        })
    }

    fn implementation(&self) -> Option<Implementation> {
        self.implementation.get().cloned()
    }

    fn close(&self) {
        self.calls.close();
        if self.out.try_send(Message::Close(None)).is_err() {
            // 已经断了，或者写队列满了：写的一头随连接一起放下。
            tracing::debug!(target: miyu_im::TARGET, "old connection not told to close");
        }
    }
}

#[cfg(test)]
mod tests;
