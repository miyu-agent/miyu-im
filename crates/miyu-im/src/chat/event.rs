//! 交给宿主的事和发出去的目标（施工 I-2 上从 `onebot.rs` 挪来）：一条消息、一次撤回、禁言和解禁、连上、好友请求和群邀请；
//! 发给谁、前面带的引用和 @。

use serde_json::Value;

use super::{Id, Rank, Segments};

/// 一条消息：私聊的或者群里的（第 5 条，「群消息」第 1 条）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posted {
    /// 收到它的机器人的号（事件的 `self_id`）。
    pub bot: Id,
    /// 发的人的号（`user_id`）。
    pub user: Id,
    /// 消息编号（`message_id`）：拼进命令编号，断线重发、平台重发都只收一次（第 8 条）。
    pub message_id: i64,
    /// 平台给的时刻（事件的 `time`，整数秒）：也拼进命令编号（第 8 条）。没带、读不出的是 0（「施工时定的」第 20 条）：消息
    /// 照送，去重退回只看消息编号。
    pub time: i64,
    /// 发的人此刻叫什么：`sender` 的群名片，空白的取昵称（驱动认，施工 O-22）；都没有的是空的。照原样，没洗。
    pub name: Option<String>,
    /// 发的人在群里的身份：`sender.role`（驱动认，施工 O-31）；私聊的、没带的、认不出的是空的。
    pub rank: Option<Rank>,
    /// 读出来的字（第 6 条）：只有 `text` 段，照原样，没去掉首尾空白。私聊照它送。
    pub text: String,
    /// 认出来的段（施工 O-22，驱动认）：群的正文照它写；私聊取引用和带的东西。
    pub segments: Segments,
}

/// 一次撤回（「撤回」，施工 O-22）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recall {
    /// 收到它的机器人的号。
    pub bot: Id,
    /// 群号：私聊的撤回是空的。
    pub group: Option<Id>,
    /// 被撤的那一条是谁发的（`user_id`）：私聊里就是对方。
    pub user: Id,
    /// 谁撤的：群里是 `operator_id`，私聊是 `user_id`。
    pub by: Id,
    /// 被撤的那一条的消息编号。
    pub message_id: i64,
}

/// 交给跟核心的那一头的一件事。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// 一条私聊。
    Private(Posted),
    /// 一条群消息（施工 O-22）。
    Group {
        /// 群号（`group_id`）。
        group: Id,
        /// 这条消息。
        posted: Posted,
    },
    /// 补拉回来的一条群消息（施工 I-15，设计 18 第十一节）：桥断线时漏掉、重连以后驱动从平台补回来的，只进聊天记录，不判、不开
    /// 回合。
    Backfilled {
        /// 群号。
        group: Id,
        /// 这条消息。
        posted: Posted,
    },
    /// 一次撤回（施工 O-22）。
    Recalled(Recall),
    /// 她在群里被禁言了（施工 O-25 中，「出站队列」第 7 条）。
    Muted {
        /// 收到通知的机器人的号：被禁言的就是它。
        bot: Id,
        /// 群号。
        group: Id,
        /// 禁几秒，大于 0。
        seconds: u64,
        /// 谁禁的（`operator_id`，施工 O-50）；没带的是空的。
        operator: Option<Id>,
    },
    /// 她在群里被解禁了（施工 O-25 中）：`lift_ban`，或者禁 0 秒。
    Unmuted {
        /// 收到通知的机器人的号。
        bot: Id,
        /// 群号。
        group: Id,
        /// 谁解的（施工 O-50）；没带的是空的。
        operator: Option<Id>,
    },
    /// 一个机器人号连上了（施工 O-25 中，「怎么走」第 2 条）：号认出来的那一刻（`X-Self-ID`、第一条事件），由连接那一头交，
    /// 不是 NapCat 的一帧。排着的照先后发。
    Connected {
        /// 机器人的号。
        bot: Id,
    },
    /// 有人要加她好友（施工 O-27，「好友请求」）：`post_type = request`、`request_type = friend`。
    Befriend {
        /// 收到请求的机器人的号：同意时经它的连接回。
        bot: Id,
        /// 要加她的人的号。
        user: Id,
        /// 平台给这次请求的标记：同意时原样交回（`set_friend_add_request` 的 `flag`）。不进运行日志。
        flag: String,
    },
    /// 驱动带的插件给场所会话记一句（施工 Q-3，`Gate::note`）：im 套上插件记事实的外壳（`facts/plugin-note.txt`）记进那个场所的
    /// 主会话，不开回合。
    Noted {
        /// 收到那件事的机器人的号。
        bot: Id,
        /// 哪个场所。
        to: To,
        /// 事实的种类（当标签名）。
        kind: &'static str,
        /// 换好的那一句。
        text: String,
    },
    /// 驱动带的插件请 im 替它问一次模型（施工 Q-7，`Gate::ask`）：用那个场所的判断模型（会话的旁路请求先照它），问完把字或者原因
    /// 交给 `reply`。
    Asked {
        /// 哪个场所。
        to: To,
        /// `model.call` 的参数（`purpose`、`messages`、`max_tokens`）：`model` 由 im 填。
        request: Value,
        /// 答到哪。
        reply: Replier,
    },
    /// 群里的一件事（设计 04，施工 C-1 上补）：别人被禁言、解禁，有人被踢、退群、进群。只记进聊天记录；她自己被禁言、解禁照
    /// [`Event::Muted`]、[`Event::Unmuted`]。
    Happened(Happened),
    /// 群里有人的群名片改了（施工 I-38，`group_card`）：群成员的名字缓存跟着改，改的是她自己的，下一条场所消息的 `my_name` 就带新的。
    Renamed {
        /// 收到通知的机器人的号。
        bot: Id,
        /// 群号。
        group: Id,
        /// 改了群名片的人。
        user: Id,
        /// 新的群名片，照原样；清掉了（显示回昵称）的是空的。
        card: Option<String>,
    },
    /// 有人邀请她进群（施工 O-27）：`request_type = group`、`sub_type = invite`。只记一行，放着。
    Invited {
        /// 收到邀请的机器人的号。
        bot: Id,
        /// 邀请她进的群。
        group: Id,
        /// 邀请的人。
        user: Id,
    },
}

/// 群里的一件事（[`Event::Happened`]）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Happened {
    /// 收到通知的机器人的号。
    pub bot: Id,
    /// 群号。
    pub group: Id,
    /// 哪一种。
    pub change: GroupChange,
    /// 对谁：被禁言、被踢、退群、进群的那个人。
    pub user: Id,
    /// 谁做的（禁言、解禁、踢人的管理员，进群时同意、邀请的人）；没带的、0 是空的。
    pub operator: Option<Id>,
    /// 禁言几秒（[`GroupChange::Muted`]）。
    pub seconds: Option<u64>,
}

/// 群里的一件事是哪一种。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupChange {
    /// 被禁言。
    Muted,
    /// 被解禁。
    Unmuted,
    /// 被踢出群。
    Kicked,
    /// 自己退群。
    Left,
    /// 进群。
    Joined,
}

impl GroupChange {
    /// 记进聊天记录的写法。
    pub fn as_str(self) -> &'static str {
        match self {
            GroupChange::Muted => "muted",
            GroupChange::Unmuted => "unmuted",
            GroupChange::Kicked => "kicked",
            GroupChange::Left => "left",
            GroupChange::Joined => "joined",
        }
    }
}

/// 发给谁：私聊的对方、群（第 10 条，「群消息」第 7 条）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum To {
    /// 和号是它的人的私聊。
    Private(Id),
    /// 群号是它的群（施工 O-22）。
    Group(Id),
}

/// 一段前面带的引用和 @（施工 O-25 上，`onebot.md`「群里怎么叫她」第 9 条）：引用的那一条的平台编号、@ 的号，照平台给的原样
/// 写成字（旧版的教训：引用的编号原样还回去，自作聪明换写法的，对端会不声不响地丢掉引用）。都没有的是不带。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lead {
    /// 引用的那一条的平台编号：`reply` 段。
    pub reply: Option<String>,
    /// @ 的号：`at` 段，后面跟一个空格的文字段。
    pub at: Option<String>,
    /// 她用 `mention` 指定要 @ 的号（施工 O-54 后的 O-55）：照先后接在 `at` 后面，和它重的不再写。
    pub more: Vec<String>,
}

/// 问一次模型的答往哪交（[`Event::Asked`]）：只交一次。比、抄都照同一个交的地方算（事件要能比、能抄）。
#[derive(Clone)]
pub struct Replier(std::sync::Arc<std::sync::Mutex<Option<Answered>>>);

/// 等着答的一头（[`Replier`]）。
type Answered = tokio::sync::oneshot::Sender<Result<String, String>>;

impl Replier {
    /// 一对：交的一头和等的一头。
    pub fn pair() -> (
        Replier,
        tokio::sync::oneshot::Receiver<Result<String, String>>,
    ) {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        (
            Replier(std::sync::Arc::new(std::sync::Mutex::new(Some(sender)))),
            receiver,
        )
    }

    /// 交答：模型回的字，或者没问成的原因。交过的、等的一头不等了的不管。
    pub fn send(&self, answer: Result<String, String>) {
        let taken = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(sender) = taken
            && sender.send(answer).is_err()
        {
            // 等的一头不等了：不要紧。
        }
    }
}

impl PartialEq for Replier {
    fn eq(&self, other: &Replier) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Replier {}

impl std::fmt::Debug for Replier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Replier")
    }
}
