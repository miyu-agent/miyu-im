//! OneBot（QQ）给 AI 的工具（施工 T-1 起，2026-10-20 项目主人定：不同平台的工具最终调的接口可能完全不一样，平台的工具在驱动里，
//! im 不提供工具）：照 im 的口子（`miyu_im::toolset::ToolSet`）交一套，说明在包文件夹的 `data/onebot/tools/`，答的话在
//! `data/onebot/tool-results/`。im 照会话填好场景交过来，这里认参数、去 QQ 上办。
//!
//! 施工 T-1 挪过来的是头像、群成员（`lookup`，施工 O-54），T-2 是撤回、禁言、戳一戳（`acting`、`target`，施工 O-31），T-3 是取图片
//! 文件（`fetch`，施工 O-33），T-4 是不回复、@ 人、主动发消息（施工 O-26、O-55、O-56；不回复、主动发消息由 im 照它的口子办，
//! `mention` 认成员在这里）。挪完 im 不提供工具。

use std::time::Duration;

use miyu_client::tooltext::Access;
use miyu_im::toolset::{Call, Decl, Effect, Planned, ToolSet, send_request};
use miyu_im::tuning::Tuning;
use serde_json::{Map, Value};

mod acting;
mod fetch;
mod lookup;
mod mention;
mod target;

/// 「这一轮不说话」（施工 O-26，「提供者和不说话」第 3 条）：im 的读的一头当场答，她的回复里有这件的调用块，这一轮就不发了。
pub const SKIP_REPLY: &str = "skip_reply";

/// @ 指定的人（施工 O-55）：只给群。
pub const MENTION: &str = "mention";

/// 主动发消息（施工 O-56）：发给她的好友、她在的群。
pub const CHAT_SEND: &str = "chat_send";

/// 撤回（施工 O-31）：撤叫她做的那条引用的那一条，或者她给的编号。
pub const RECALL: &str = "recall";

/// 禁言（施工 O-31）：只给群。
pub const MUTE: &str = "mute";

/// 戳一戳（施工 O-31）。
pub const POKE: &str = "poke";

/// 取图、视频、文件（施工 O-33）：私聊、群都给。
pub const FETCH_MEDIA: &str = "fetch_media";

/// 取头像（施工 O-54）：一个人的、一个群的，存进工作区。
pub const AVATAR: &str = "avatar";

/// 查群成员（施工 O-54）：只给群。
pub const MEMBERS: &str = "members";

/// 私聊、群都给。
const VENUES: &[&str] = &["private", "group"];

/// 有哪几件，照登记的先后。撤回、禁言、戳一戳在平台上留下后果，`avatar` 在平台上取东西、写进这个会话的工作区，都是在场所里做的事
/// （`venue`）；`members` 只读。
const TOOLS: [Decl; 9] = [
    Decl {
        name: SKIP_REPLY,
        access: Access::Read,
        venues: VENUES,
    },
    Decl {
        name: RECALL,
        access: Access::Venue,
        venues: VENUES,
    },
    Decl {
        name: MUTE,
        access: Access::Venue,
        venues: &["group"],
    },
    Decl {
        name: POKE,
        access: Access::Venue,
        venues: VENUES,
    },
    Decl {
        name: FETCH_MEDIA,
        access: Access::Venue,
        venues: VENUES,
    },
    Decl {
        name: AVATAR,
        access: Access::Venue,
        venues: VENUES,
    },
    Decl {
        name: MEMBERS,
        access: Access::Read,
        venues: &["group"],
    },
    Decl {
        name: MENTION,
        access: Access::Venue,
        venues: &["group"],
    },
    Decl {
        name: CHAT_SEND,
        access: Access::Venue,
        venues: VENUES,
    },
];

/// 答的话：名字和认的字段（`failed`、`unanswered`、`unreachable` 几件共用）。
const ANSWERS: [(&str, &[&str]); 26] = [
    ("skipped", &[]),
    ("mentioned", &["who"]),
    ("not-members", &["who"]),
    ("sent", &[]),
    ("no-item", &["count"]),
    ("not-fetchable", &[]),
    ("too-many", &["count"]),
    ("recalled", &[]),
    ("muted", &["who", "duration"]),
    ("unmuted", &["who"]),
    ("poked", &["who"]),
    ("no-quote", &[]),
    ("many-targets", &[]),
    ("bad-seconds", &["min", "max"]),
    ("admin-off", &[]),
    ("not-found", &[]),
    ("unknown", &["name"]),
    ("saved", &["path"]),
    ("unfetched", &["detail"]),
    ("not-saved", &["detail"]),
    ("members", &["shown", "total", "list"]),
    ("no-members", &[]),
    ("no-target", &[]),
    ("failed", &["detail"]),
    ("unanswered", &[]),
    ("unreachable", &[]),
];

/// 照 `bridge.json` 的参数造的一套。
#[derive(Debug, Clone)]
pub struct QqTools {
    /// 头像的地址模板：一个人的、一个群的（`avatar_user_url`、`avatar_group_url`，`{id}` 换成号）。
    pub(crate) avatars: (String, String),
    /// 群成员最多列几个（`members_shown`）。
    pub(crate) members: usize,
    /// 下东西最多等多久（`fetch_seconds`）。
    pub(crate) wait: Duration,
    /// `fetch_media` 一轮最多取几次（`fetch_per_turn`）。
    pub(crate) per_turn: usize,
}

impl QqTools {
    /// 照 `bridge.json` 的参数 `tuning` 造。
    pub fn new(tuning: &Tuning) -> QqTools {
        QqTools {
            avatars: (
                tuning.avatar_user_url.clone(),
                tuning.avatar_group_url.clone(),
            ),
            members: tuning.members_shown,
            wait: tuning.fetch(),
            per_turn: tuning.fetch_per_turn,
        }
    }
}

impl ToolSet for QqTools {
    fn dir(&self) -> &'static str {
        "onebot"
    }

    fn tools(&self) -> &'static [Decl] {
        &TOOLS
    }

    fn answers(&self) -> &'static [(&'static str, &'static [&'static str])] {
        &ANSWERS
    }

    fn silence(&self) -> Option<&'static str> {
        Some(SKIP_REPLY)
    }

    fn offers(&self, tool: &str, config: &Map<String, Value>) -> bool {
        // 群管理的工具照它的开关（施工 Q-1）。
        tool != MUTE || crate::admin::offers_mute(config)
    }

    fn plan(&self, call: Call) -> Planned {
        match call.tool.as_str() {
            RECALL | MUTE | POKE => acting::acting(call),
            FETCH_MEDIA => fetch::fetch_media(call, self.per_turn, self.wait),
            MENTION => mention::mention(call),
            CHAT_SEND => match send_request(&call.args) {
                Some((to, text)) => Planned::Host(Effect::Send { to, text }),
                None => Planned::Now(call.answers.say("no-target", &[], true)),
            },
            // 「这一轮不说话」im 的读的一头当场答，到不了这里；照说不会。
            SKIP_REPLY => Planned::Now(call.answers.say("skipped", &[], false)),
            AVATAR => lookup::avatar(self, call),
            MEMBERS => lookup::members(self, call),
            // im 只交这一套里有的：照说不会。
            _ => Planned::Now(call.answers.say("unreachable", &[], true)),
        }
    }
}
