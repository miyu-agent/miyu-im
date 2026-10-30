//! OneBot 的一条连接（施工 I-2 上）：宿主要做的事（[`Link`]）照 OneBot v11 拼成动作，放进这条反向 WebSocket 的写队列，回应照
//! `echo` 配对（[`Calls`]），读成宿主认的样子（`replies`）。

use std::sync::OnceLock;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use super::media::{get_media, get_msg};
use super::replies::{
    self, FILE_URL, FOLDER_FILES, FRIEND_LIST, GROUP_INFO, GROUP_LIST, LOGIN_INFO, MEMBER_LIST,
    ROOT_FILES,
};
use super::{
    Calls, MEMBER_INFO, delete_msg, emoji_like, forward_messages, forward_to, friend_add,
    get_forward_msg, group_add, group_ban, group_kick, image_to, member_info, message_id,
    message_to, node, picture_to, poke, upload_to,
};
use miyu_im::chat::{
    self, CallError, Face, Fetch, Fetched, Friend, Group, GroupFiles, Id, Implementation, Link,
    Located, Member, Node, Outgoing, Reply, Source, To,
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
    /// 头像的地址模板：人的、群的（`bridge.json` 的 `avatar_user_url`、`avatar_group_url`，`{id}` 换成号；施工 I-82 从宿主挪来）。
    pub avatars: (String, String),
    /// 从哪个端口连进来的（施工 I-81 下，[`Wire::on_port`]）。
    pub listen: Option<u16>,
}

impl Wire {
    /// 一条新的连接：往 `out` 写，一个回应最多等 `timeout`；头像照地址模板 `avatars`（人的、群的）。
    pub fn new(out: mpsc::Sender<Message>, timeout: Duration, avatars: (String, String)) -> Wire {
        Wire {
            out,
            calls: Calls::new(timeout),
            implementation: OnceLock::new(),
            avatars,
            listen: None,
        }
    }

    /// 记下它是从端口 `port` 连进来的。
    pub fn on_port(mut self, port: u16) -> Wire {
        self.listen = Some(port);
        self
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
                Ok(message_id(&reply["data"]["message_id"]))
            });
            Ok(sent)
        })
    }

    fn recall<'a>(&'a self, _: &'a To, message: &'a str) -> Reply<'a, ()> {
        self.done(delete_msg(message))
    }

    fn mute<'a>(&'a self, group: &'a Id, user: &'a Id, seconds: u64) -> Reply<'a, ()> {
        self.done(group_ban(group, user, seconds))
    }

    fn kick<'a>(&'a self, group: &'a Id, user: &'a Id, block: bool) -> Reply<'a, ()> {
        self.done(group_kick(group, user, block))
    }

    fn poke<'a>(&'a self, to: To, user: &'a Id) -> Reply<'a, ()> {
        self.done(poke(to, user))
    }

    fn react<'a>(&'a self, _: &'a To, message: &'a str, emoji: &'a str, on: bool) -> Reply<'a, ()> {
        self.done(emoji_like(message, emoji, on))
    }

    fn befriend<'a>(&'a self, flag: &'a str) -> Reply<'a, ()> {
        self.done(friend_add(flag))
    }

    fn join_request<'a>(&'a self, flag: &'a str, approve: bool, reason: &'a str) -> Reply<'a, ()> {
        self.done(group_add(flag, approve, reason))
    }

    fn message<'a>(&'a self, _: &'a To, id: &'a str) -> Reply<'a, Fetched> {
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

    fn members<'a>(&'a self, group: &'a Id) -> Reply<'a, Vec<Member>> {
        self.read((MEMBER_LIST, json!({"group_id": group})), replies::members)
    }

    fn member<'a>(&'a self, group: &'a Id, user: &'a Id) -> Reply<'a, Member> {
        self.read((MEMBER_INFO, member_info(group, user)), move |data| {
            let mut member = replies::member(data).unwrap_or(Member {
                user: user.clone(),
                name: None,
                rank: None,
                title: None,
            });
            member.user = user.clone();
            member
        })
    }

    fn groups(&self) -> Reply<'_, Vec<Group>> {
        self.read((GROUP_LIST, json!({})), replies::groups)
    }

    fn friends(&self) -> Reply<'_, Vec<Friend>> {
        self.read((FRIEND_LIST, json!({})), replies::friends)
    }

    fn group_files<'a>(&'a self, group: &'a Id, folder: Option<&'a str>) -> Reply<'a, GroupFiles> {
        let action = match folder {
            Some(folder) => (
                FOLDER_FILES,
                json!({"group_id": group, "folder_id": folder}),
            ),
            None => (ROOT_FILES, json!({"group_id": group})),
        };
        self.read(action, replies::group_files)
    }

    fn group_file_url<'a>(&'a self, group: &'a Id, file: &'a str) -> Reply<'a, String> {
        let wanted = (FILE_URL, json!({"group_id": group, "file_id": file}));
        Box::pin(async move {
            self.read(wanted, replies::file_url)
                .await?
                .ok_or_else(|| CallError::Failed {
                    said: "no url".to_string(),
                    message: String::new(),
                })
        })
    }

    fn group_name<'a>(&'a self, group: &'a Id) -> Reply<'a, Option<String>> {
        self.read(
            (GROUP_INFO, json!({"group_id": group})),
            replies::group_name,
        )
    }

    fn nickname(&self) -> Reply<'_, Option<String>> {
        self.read((LOGIN_INFO, json!({})), replies::nickname)
    }

    fn avatar<'a>(&'a self, face: &'a Face) -> Reply<'a, Option<Source>> {
        let url = avatar_url(&self.avatars, face).map(Source::Url);
        Box::pin(async move { Ok(url) })
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

    fn listen(&self) -> Option<u16> {
        self.listen
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

/// QQ 头像的地址（施工 I-82）：照模板 `avatars`（人的、群的）把 `{id}` 换成号；号要是正整数，模板空着的没有。
fn avatar_url((user, group): &(String, String), face: &Face) -> Option<String> {
    let (template, id) = match face {
        Face::User(id) => (user, id),
        Face::Group(id) => (group, id),
    };
    let positive = id.number().is_some_and(|number| number > 0);
    (positive && !template.is_empty()).then(|| template.replace("{id}", id.as_str()))
}
