//! OneBot v11 这一头（`onebot.md` 第一条「怎么走」第 4 到 6 条、第 10 条，「群消息」「撤回」「出站队列」「好友请求」）：NapCat
//! 发来的一帧认成什么（回应、私聊、群消息、撤回、她被禁言和解禁（施工 O-25 中）、好友请求和群邀请（施工 O-27）、别的事件），
//! 私聊里的文字怎么读出来（`text`），
//! 消息段怎么认（`segments`，施工 O-22），群成员叫什么（`members`，施工 O-22），发出去的动作和回应怎么照 `echo` 配对
//! （`calls`），`send_private_msg`、`send_group_msg`（施工 O-25 上：第一段能带引用和 @）、`delete_msg`（施工 O-25 上）、
//! `set_msg_emoji_like`（施工 O-25 下）、`set_friend_add_request`（施工 O-27）、`set_group_ban`、`group_poke`、`friend_poke`
//! （施工 O-31）写成什么样；取带的东西的 `get_msg`、`get_image`、`get_file`（施工 O-33，`media`）；合并转发里的消息
//! `get_forward_msg` 和节点怎么读（施工 O-34，`forwards`）。
//!
//! 号（机器人的号、对方的号、消息编号）和时刻照 OneBot 是整数；有的实现写成字符串，也认。
//!
//! 平台的名字 [`PLATFORM`] 只写在这里：场所、平台上的人、命令编号都照它拼（第 7、8 条，拼法在宿主的 [`Platform`]）。

mod calls;
mod forwards;
mod happenings;
pub mod media;
mod members;
pub(crate) mod reactions;
mod renamed;
mod replies;
mod segments;
mod send;
mod text;
mod wire;

use serde_json::{Value, json};

pub use calls::{CallError, Calls, DETAIL, Pending, failed, said};
pub use forwards::{Node, forward_messages, get_forward_msg, node};
pub use members::{
    MEMBER_INFO, Members, Rank, display_name, fresh_member, fresh_member_info, member_info, rank_of,
};
pub use miyu_im::chat::{Event, Id, Lead, Platform, Posted, Recall, To};
pub use segments::{FORWARD, Fetch, Forwarded, Media, MediaKind, Piece, Segments, segments};
pub use send::{forward_to, image_to, message_to, picture_to};
pub use text::text_of;
pub use wire::Wire;

/// 平台：场所、平台上的人、命令编号的头一段是 `qq`（第 7、8 条，拼法在 [`Platform`]）。
pub const PLATFORM: Platform = Platform::new("qq");

/// NapCat 发来的一帧。
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// 动作的回应：带 `echo`、不是事件。
    Reply(Value),
    /// 一条消息、一次撤回、禁言和解禁（施工 O-25 中）、好友请求和群邀请（施工 O-27）。
    Event(Event),
    /// 别的事件（别的通知、请求、心跳、生命周期、机器人自己发的……）或者读不懂的：`post_type`，没有的是空字。
    Other(String),
}

/// 认一帧（第 5 条）。
pub fn read(frame: Value) -> Frame {
    let kind = frame["post_type"].as_str().unwrap_or_default().to_string();
    if kind.is_empty() && frame.get("echo").is_some() {
        return Frame::Reply(frame);
    }
    let event = match kind.as_str() {
        "message" => posted(&frame),
        "notice" => recall(&frame)
            .or_else(|| ban(&frame))
            .or_else(|| happenings::happened(&frame))
            .or_else(|| renamed::renamed(&frame)),
        "request" => request(&frame),
        _ => None,
    };
    match event {
        Some(event) => Frame::Event(event),
        None => Frame::Other(kind),
    }
}

/// 一条消息：私聊、群的，号都带得出、不是机器人自己发的（「施工时定的」第 69 条）才是。
fn posted(frame: &Value) -> Option<Event> {
    let (bot, user, message_id) = (
        id(&frame["self_id"])?,
        id(&frame["user_id"])?,
        number(&frame["message_id"])?,
    );
    if user == bot {
        return None;
    }
    let group = match frame["message_type"].as_str() {
        Some("private") => None,
        Some("group") => Some(id(&frame["group_id"])?),
        _ => return None,
    };
    let posted = Posted {
        bot,
        user,
        message_id,
        time: number(&frame["time"]).unwrap_or(0),
        name: display_name(&frame["sender"]),
        rank: rank_of(&frame["sender"]),
        text: text_of(&frame["message"]),
        segments: segments(&frame["message"]),
    };
    Some(match group {
        Some(group) => Event::Group { group, posted },
        None => Event::Private(posted),
    })
}

/// 一次撤回：群的（`group_recall`）、私聊的（`friend_recall`），号都带得出的才是。
fn recall(frame: &Value) -> Option<Event> {
    let (bot, user, message_id) = (
        id(&frame["self_id"])?,
        id(&frame["user_id"])?,
        number(&frame["message_id"])?,
    );
    let (group, by) = match frame["notice_type"].as_str() {
        Some("group_recall") => (Some(id(&frame["group_id"])?), id(&frame["operator_id"])?),
        Some("friend_recall") => (None, user.clone()),
        _ => return None,
    };
    Some(Event::Recalled(Recall {
        bot,
        group,
        user,
        by,
        message_id,
    }))
}

/// 禁言、解禁（`group_ban`，施工 O-25 中，「出站队列」第 7 条）：禁的是她（`user_id` 等于 `self_id`）的才是，全员禁言
/// （`user_id` 是 0）、禁别人的不是。`ban` 带的 `duration`（秒）大于 0 是禁言，0 是解禁；`lift_ban` 是解禁；没带 `duration`、
/// 负的、别的 `sub_type` 不是（「施工时定的」第 121 条）。
fn ban(frame: &Value) -> Option<Event> {
    if frame["notice_type"] != "group_ban" {
        return None;
    }
    let (bot, user, group) = (
        id(&frame["self_id"])?,
        id(&frame["user_id"])?,
        id(&frame["group_id"])?,
    );
    if user != bot {
        return None;
    }
    let seconds = match frame["sub_type"].as_str()? {
        "ban" => u64::try_from(number(&frame["duration"])?).ok()?,
        "lift_ban" => 0,
        _ => return None,
    };
    // 谁禁的、谁解的（施工 O-50）：0 和没带的当不知道。
    let operator = positive(&frame["operator_id"]);
    Some(match seconds {
        0 => Event::Unmuted {
            bot,
            group,
            operator,
        },
        seconds => Event::Muted {
            bot,
            group,
            seconds,
            operator,
        },
    })
}

/// 请求（施工 O-27，「好友请求」）：加好友的（`friend`，带得出号、不空的标记）、邀请她进群的（`group` 里 `sub_type` 是
/// `invite`，带得出号、群号）才是；入群申请（`sub_type` 是 `add`，随入群审批那一步）、别的不是。
fn request(frame: &Value) -> Option<Event> {
    let (bot, user) = (id(&frame["self_id"])?, id(&frame["user_id"])?);
    match frame["request_type"].as_str()? {
        "friend" => {
            let flag = frame["flag"].as_str().filter(|flag| !flag.is_empty())?;
            Some(Event::Befriend {
                bot,
                user,
                flag: flag.to_string(),
            })
        }
        "group" if frame["sub_type"] == "invite" => Some(Event::Invited {
            bot,
            group: id(&frame["group_id"])?,
            user,
        }),
        _ => None,
    }
}

/// 通知、请求是哪个场所的（施工 I-8 上：挂接点照场所开关问插件）：带群号的是那个群，不然带对方的号的是和他的私聊；都没有的
/// 是空的。
pub fn event_to(frame: &Value) -> Option<To> {
    id(&frame["group_id"])
        .map(To::Group)
        .or_else(|| id(&frame["user_id"]).map(To::Private))
}

/// 事件里机器人的号（`self_id`）：连进来时没带 `X-Self-ID` 的，照第一条事件的认（第 2 条）。
pub fn self_id(frame: &Value) -> Option<Id> {
    id(&frame["self_id"])
}

/// 一个号（群号、QQ 号、机器人的号）：同 [`number`]，交给宿主的是字（施工 I-79：宿主里的号是字）。
pub fn id(value: &Value) -> Option<Id> {
    number(value).map(Id::from)
}

/// 一个正的号：0 和没带的当不知道（谁禁的、谁解的）。
pub fn positive(value: &Value) -> Option<Id> {
    number(value).filter(|number| *number > 0).map(Id::from)
}

/// 一个号或者时刻：整数，或者写成整数的字符串。
pub fn number(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
}

/// 把一份文件（`file` 写成 `base64://…`，文件名 `name`）传给 `to` 的动作和参数：`upload_private_file {user_id, file, name}`、
/// `upload_group_file {group_id, file, name}`（施工 O-46，「展示」第 2 条；NapCat 的 `file` 认本机路径、网址和 `base64://`）。
pub fn upload_to(to: To, file: &str, name: &str) -> (&'static str, Value) {
    match to {
        To::Private(user) => (
            "upload_private_file",
            json!({"user_id": user, "file": file, "name": name}),
        ),
        To::Group(group) => (
            "upload_group_file",
            json!({"group_id": group, "file": file, "name": name}),
        ),
    }
}

/// 撤回平台编号是 `message_id` 的那一条的动作和参数：`delete_msg {message_id}`（施工 O-25 上，「斜杠命令」第 7 条）。编号照原样
/// 写成字（施工 O-31：原样还回去，同 [`emoji_like`]；I-82 起回执的编号也是字），NapCat 两样都收（`DeleteMsg.ts`）。
pub fn delete_msg(message_id: &str) -> (&'static str, Value) {
    ("delete_msg", json!({"message_id": message_id}))
}

/// NapCat 回的消息编号（`data.message_id`）：数、字都认，交回字；没有的、空的是空的（施工 I-82）。
pub fn message_id(value: &Value) -> Option<String> {
    match value {
        Value::Number(number) => Some(number.to_string()),
        Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_string()),
        _ => None,
    }
}

/// 在群 `group` 里禁言号是 `user` 的人 `seconds` 秒的动作和参数（0 是解禁）：`set_group_ban {group_id, user_id, duration}`（施工
/// O-31，「平台工具（一）」第 1 条）。
pub fn group_ban(group: &Id, user: &Id, seconds: u64) -> (&'static str, Value) {
    (
        "set_group_ban",
        json!({"group_id": group, "user_id": user, "duration": seconds}),
    )
}

/// 把号是 `user` 的人移出群 `group` 的动作和参数：`set_group_kick {group_id, user_id, reject_add_request}`，`block` 的以后不收他的
/// 加群申请（施工 Q-2）。
pub fn group_kick(group: &Id, user: &Id, block: bool) -> (&'static str, Value) {
    (
        "set_group_kick",
        json!({"group_id": group, "user_id": user, "reject_add_request": block}),
    )
}

/// 戳一戳号是 `user` 的人的动作和参数（施工 O-31）：在 `to` 那个群里 `group_poke {group_id, user_id}`，私聊里
/// `friend_poke {user_id}`（NapCat 的 `SendPoke.ts`：群号没写的是私聊）。
pub fn poke(to: To, user: &Id) -> (&'static str, Value) {
    match to {
        To::Group(group) => ("group_poke", json!({"group_id": group, "user_id": user})),
        To::Private(_) => ("friend_poke", json!({"user_id": user})),
    }
}

/// 在平台编号是 `message` 的那一条上贴（`set` 是真）、摘（假）表情 `emoji` 的动作和参数：`set_msg_emoji_like {message_id,
/// emoji_id, set}`（施工 O-25 下，「贴表情」第 2 条）。编号、表情都原样写成字：NapCat 数和字都收（`SetMsgEmojiLike.ts`），
/// 编号原样还回去，不换写法（「施工时定的」第 109、132 条）。
pub fn emoji_like(message: &str, emoji: &str, set: bool) -> (&'static str, Value) {
    (
        "set_msg_emoji_like",
        json!({"message_id": message, "emoji_id": emoji, "set": set}),
    )
}

/// 处理标记是 `flag` 的那次加群申请的动作和参数：`set_group_add_request {flag, sub_type: "add", approve, reason}`（施工 Q-7）；同意的
/// 不带理由。
pub fn group_add(flag: &str, approve: bool, reason: &str) -> (&'static str, Value) {
    let mut params = json!({"flag": flag, "sub_type": "add", "approve": approve});
    if !approve {
        params["reason"] = json!(reason);
    }
    ("set_group_add_request", params)
}

/// 同意标记是 `flag` 的那次好友请求的动作和参数：`set_friend_add_request {flag, approve: true}`（施工 O-27，「好友请求」）。
/// 标记照平台给的原样交回。
pub fn friend_add(flag: &str) -> (&'static str, Value) {
    (
        "set_friend_add_request",
        json!({"flag": flag, "approve": true}),
    )
}
